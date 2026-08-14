use crate::device::DeviceManager;
use crate::models::{AuthSession, DeviceProfile};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_DISPENSERS: &[&str] = &[
    "https://auroraoss.com/api/auth",
    "https://dispenser.auroraoss.com/api/auth",
];

#[derive(Clone)]
pub struct AuthManager {
    client: Client,
    device: DeviceProfile,
    custom_dispenser: Option<String>,
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl AuthManager {
    pub fn new(device: Option<DeviceProfile>, custom_dispenser: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let device = device.unwrap_or_else(DeviceManager::pixel_7a);
        Self {
            client,
            device,
            custom_dispenser,
        }
    }

    /// Path to the local session file: ~/.config/gplay-cli/session.json
    pub fn session_file_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
            .context("Failed to determine user home directory")?;
        let config_dir = home.join(".config").join("gplay-cli");
        fs::create_dir_all(&config_dir)?;
        let path = config_dir.join("session.json");
        if !path.exists() {
            let legacy = home.join(".config").join("gplay").join("session.json");
            if legacy.exists() {
                let _ = fs::copy(&legacy, &path);
            }
        }
        Ok(path)
    }

    /// Load cached session from disk if present and not older than 12 hours
    pub fn load_cached_session() -> Option<AuthSession> {
        let path = Self::session_file_path().ok()?;
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(path).ok()?;
        let session: AuthSession = serde_json::from_str(&content).ok()?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

        // 12 hours validity window
        if now.saturating_sub(session.created_at) < 12 * 3600 {
            Some(session)
        } else {
            None
        }
    }

    /// Save session to disk
    pub fn save_session(session: &AuthSession) -> Result<()> {
        let path = Self::session_file_path()?;
        let json = serde_json::to_string_pretty(session)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Clear cached session
    pub fn clear_session() -> Result<()> {
        let path = Self::session_file_path()?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Obtains a valid AuthSession (from cache or newly dispensed)
    pub async fn get_session(&self, force_refresh: bool) -> Result<AuthSession> {
        // Check environment variable overrides first
        let email_opt = std::env::var("GPLAY_EMAIL").or_else(|_| std::env::var("GDOWN_EMAIL"));
        let token_opt =
            std::env::var("GPLAY_AUTH_TOKEN").or_else(|_| std::env::var("GDOWN_AUTH_TOKEN"));
        if let (Ok(email), Ok(token)) = (email_opt, token_opt) {
            let gsf_id = std::env::var("GPLAY_GSF_ID")
                .or_else(|_| std::env::var("GDOWN_GSF_ID"))
                .unwrap_or_else(|_| "3893fb8c620e278e".to_string());
            let ua = format!(
                "Android-Finsky/{} (api=3,versionCode={},sdk={},device={},hardware={},product={},platformVersionRelease={},model={},buildId={})",
                self.device.vending_version_string,
                self.device.vending_version,
                self.device.sdk_int,
                self.device.device,
                self.device.hardware,
                self.device.product,
                self.device.release,
                self.device.model,
                self.device.id
            );
            return Ok(AuthSession {
                email,
                auth_token: token,
                gsf_id,
                user_agent: ua,
                device_config_token: None,
                device_checkin_consistency_token: None,
                dfe_cookie: None,
                dispenser_url: "custom-env".to_string(),
                created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                is_custom: true,
            });
        }

        if let Some(session) = Self::load_cached_session() {
            if session.is_custom {
                if force_refresh {
                    return Err(anyhow::anyhow!(
                        "Custom Google Account session for '{}' expired or returned unauthorized.\nPlease re-authenticate with 'gplay-cli auth login'.",
                        session.email
                    ));
                }
                return Ok(session);
            }
            if !force_refresh {
                return Ok(session);
            }
        }

        self.dispense_new_session().await
    }

    /// Authenticates with a custom user Google Account token (OAuth / AAS token),
    /// performs Android check-in, uploads device configuration, accepts Google Play TOS,
    /// and saves the custom session to ~/.config/gplay/session.json.
    pub async fn login_custom_session(&self, email: &str, token: &str) -> Result<AuthSession> {
        let clean_token = token.trim();
        let clean_email = email.trim();

        if clean_email.is_empty() || clean_token.is_empty() {
            return Err(anyhow::anyhow!(
                "Email and authentication token must not be empty."
            ));
        }

        // 1. Perform Checkin to obtain Android GSF ID & consistency token
        let (gsf_id, checkin_consistency_token) =
            match Self::perform_checkin(&self.client, &self.device).await {
                Ok((id, token)) => (id, token),
                Err(_) => ("3893fb8c620e278e".to_string(), None),
            };

        // 2. Resolve Google Play Auth Token (AC2DM exchange if oauth_token, followed by androidmarket exchange)
        let (_, play_auth_token) = Self::resolve_google_play_auth_token(
            &self.client,
            clean_email,
            clean_token,
            &gsf_id,
            &self.device,
        )
        .await
        .context("Google account authentication failed. Please verify that you provided a fresh oauth_token or AAS token.")?;

        let user_agent = format!(
            "Android-Finsky/{} (api=3,versionCode={},sdk={},device={},hardware={},product={},platformVersionRelease={},model={},buildId={})",
            self.device.vending_version_string,
            self.device.vending_version,
            self.device.sdk_int,
            self.device.device,
            self.device.hardware,
            self.device.product,
            self.device.release,
            self.device.model,
            self.device.id
        );

        // 3. Upload Device Configuration to Google Play under the user's account
        let device_config_token = Self::perform_upload_device_config(
            &self.client,
            &play_auth_token,
            &gsf_id,
            &user_agent,
            checkin_consistency_token.as_deref(),
            &self.device,
        )
        .await
        .ok()
        .flatten();

        // 4. Perform TOC & Accept TOS to obtain X-DFE-Cookie and verify credentials
        let dfe_cookie = match Self::perform_toc_and_accept_tos(
            &self.client,
            &play_auth_token,
            &gsf_id,
            &user_agent,
            device_config_token.as_deref(),
            checkin_consistency_token.as_deref(),
        )
        .await
        {
            Ok(cookie) => cookie,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to connect to Google Play with this token: {}. Please verify the token is valid.", e));
            }
        };

        let session = AuthSession {
            email: clean_email.to_string(),
            auth_token: play_auth_token,
            gsf_id,
            user_agent,
            device_config_token,
            device_checkin_consistency_token: checkin_consistency_token,
            dfe_cookie,
            dispenser_url: "custom-login".to_string(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            is_custom: true,
        };

        Self::save_session(&session)?;
        Ok(session)
    }

    /// Performs two-stage token resolution:
    /// 1. If given a browser oauth_token (from EmbeddedSetup), exchanges via AC2DM to get Master/AAS token.
    /// 2. Exchanges the AAS token for an androidmarket Google Play AuthToken.
    pub async fn resolve_google_play_auth_token(
        client: &Client,
        email: &str,
        input_token: &str,
        gsf_id: &str,
        device: &DeviceProfile,
    ) -> Result<(String, String)> {
        let clean_token = input_token.trim();
        let clean_email = email.trim();

        // Stage 1: Try AC2DM exchange (converting browser oauth_token -> AAS Token)
        let aas_token = match Self::exchange_ac2dm_token(client, clean_email, clean_token).await {
            Ok((_, token)) => token,
            Err(_) => {
                // If AC2DM was not needed or input was already an AAS token, use input directly
                clean_token.to_string()
            }
        };

        // Stage 2: Exchange AAS token -> androidmarket Play Store auth token
        let play_auth =
            match Self::exchange_aas_to_play_token(client, clean_email, &aas_token, gsf_id, device)
                .await
            {
                Ok(token) => token,
                Err(e) => {
                    // If input was already a direct Play Store Auth token
                    if clean_token.len() > 100 && !clean_token.starts_with("oauth2_4/") {
                        clean_token.to_string()
                    } else {
                        return Err(e);
                    }
                }
            };

        Ok((clean_email.to_string(), play_auth))
    }

    /// Stage 1: Calls Google's AC2DM auth service to exchange EmbeddedSetup oauth_token for AAS Master Token
    pub async fn exchange_ac2dm_token(
        client: &Client,
        email: &str,
        oauth_token: &str,
    ) -> Result<(String, String)> {
        let form = [
            ("lang", "en-US"),
            ("google_play_services_version", "19629032"),
            ("sdk_version", "28"),
            ("device_country", "us"),
            ("Email", email),
            ("service", "ac2dm"),
            ("get_accountid", "1"),
            ("ACCESS_TOKEN", "1"),
            ("callerPkg", "com.google.android.gms"),
            ("add_account", "1"),
            ("Token", oauth_token),
            ("callerSig", "38918a453d07199354f8b19af05ec6562ced5788"),
            ("droidguard_results", "null"),
        ];

        let res = client
            .post("https://android.clients.google.com/auth")
            .header("app", "com.google.android.gms")
            .header("User-Agent", "")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await?;

        let body = res.text().await.unwrap_or_default();
        let mut resolved_email = email.to_string();
        let mut token = None;

        for line in body.lines() {
            if let Some(t) = line.strip_prefix("Token=") {
                token = Some(t.trim().to_string());
            } else if let Some(e) = line.strip_prefix("Email=") {
                let e_trim = e.trim();
                if !e_trim.is_empty() {
                    resolved_email = e_trim.to_string();
                }
            }
        }

        if let Some(t) = token {
            Ok((resolved_email, t))
        } else {
            let err_desc = body
                .lines()
                .find(|l| l.starts_with("Error="))
                .unwrap_or(&body);
            Err(anyhow::anyhow!("AC2DM exchange failed: {}", err_desc))
        }
    }

    /// Stage 2: Exchanges AAS Master Token for an androidmarket Google Play AuthToken
    pub async fn exchange_aas_to_play_token(
        client: &Client,
        email: &str,
        aas_token: &str,
        gsf_id: &str,
        device: &DeviceProfile,
    ) -> Result<String> {
        let ua = format!("GoogleAuth/1.4 ({} {})", device.device, device.id);
        let form = [
            ("accountType", "HOSTED_OR_GOOGLE"),
            ("Email", email),
            ("EncryptedPasswd", aas_token),
            ("service", "androidmarket"),
            ("app", "com.android.vending"),
            ("client_sig", "38918a453d07199354f8b19af05ec6562ced5788"),
            ("callerPkg", "com.android.vending"),
            ("callerSig", "38918a453d07199354f8b19af05ec6562ced5788"),
            ("device_country", "us"),
            ("operatorCountry", "us"),
            ("lang", "en_US"),
            ("sdk_version", "34"),
            ("google_play_services_version", "240415037"),
            ("androidId", gsf_id),
        ];

        let res = client
            .post("https://android.clients.google.com/auth")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", ua)
            .header("app", "com.google.android.gms")
            .header("device", gsf_id)
            .form(&form)
            .send()
            .await?;

        let body = res.text().await.unwrap_or_default();
        for line in body.lines() {
            if let Some(token) = line.strip_prefix("Auth=") {
                return Ok(token.trim().to_string());
            }
        }

        let err_desc = body
            .lines()
            .find(|l| l.starts_with("Error="))
            .unwrap_or(&body);
        Err(anyhow::anyhow!(
            "Play Store token exchange failed: {}",
            err_desc
        ))
    }

    /// Connects to token dispenser to generate anonymous session and registers device
    pub async fn dispense_new_session(&self) -> Result<AuthSession> {
        let payload = DeviceManager::to_dispenser_payload(&self.device);

        let mut dispenser_urls = Vec::new();
        if let Some(ref custom) = self.custom_dispenser {
            dispenser_urls.push(custom.as_str());
        }
        dispenser_urls.extend_from_slice(DEFAULT_DISPENSERS);

        let mut last_error = None;

        for url in dispenser_urls {
            let res = self
                .client
                .post(url)
                .header("Content-Type", "application/json")
                .header("User-Agent", "com.aurora.store-4.5.1-70")
                .json(&payload)
                .send()
                .await;

            match res {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        last_error = Some(anyhow::anyhow!(
                            "Dispenser {} returned {}: {}",
                            url,
                            status,
                            text
                        ));
                        continue;
                    }

                    let val: Value = match response.json().await {
                        Ok(v) => v,
                        Err(e) => {
                            last_error =
                                Some(anyhow::anyhow!("Failed to parse dispenser JSON: {}", e));
                            continue;
                        }
                    };

                    let email = val
                        .get("email")
                        .and_then(|v| v.as_str())
                        .unwrap_or("anonymous@gmail.com")
                        .to_string();

                    let auth_token = match val.get("authToken").and_then(|v| v.as_str()) {
                        Some(t) => t.to_string(),
                        None => {
                            last_error =
                                Some(anyhow::anyhow!("Dispenser response missing authToken"));
                            continue;
                        }
                    };

                    let dispenser_gsf = val
                        .get("gsfId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("3893fb8c620e278e")
                        .to_string();

                    let user_agent = val
                        .get("deviceInfoProvider")
                        .and_then(|d| d.get("userAgentString"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            format!(
                                "Android-Finsky/{} (api=3,versionCode={},sdk={},device={},hardware={},product={},platformVersionRelease={},model={},buildId={})",
                                self.device.vending_version_string,
                                self.device.vending_version,
                                self.device.sdk_int,
                                self.device.device,
                                self.device.hardware,
                                self.device.product,
                                self.device.release,
                                self.device.model,
                                self.device.id
                            )
                        });

                    // 1. Perform Checkin to obtain Android GSF ID & consistency token
                    let (gsf_id, checkin_consistency_token) =
                        match Self::perform_checkin(&self.client, &self.device).await {
                            Ok((id, token)) => (id, token),
                            Err(_) => (dispenser_gsf, None),
                        };

                    // 2. Upload Device Configuration to Google Play
                    let device_config_token = Self::perform_upload_device_config(
                        &self.client,
                        &auth_token,
                        &gsf_id,
                        &user_agent,
                        checkin_consistency_token.as_deref(),
                        &self.device,
                    )
                    .await
                    .ok()
                    .flatten();

                    // 3. Perform TOC & Accept TOS to obtain X-DFE-Cookie
                    let dfe_cookie = Self::perform_toc_and_accept_tos(
                        &self.client,
                        &auth_token,
                        &gsf_id,
                        &user_agent,
                        device_config_token.as_deref(),
                        checkin_consistency_token.as_deref(),
                    )
                    .await
                    .ok()
                    .flatten();

                    let session = AuthSession {
                        email,
                        auth_token,
                        gsf_id,
                        user_agent,
                        device_config_token,
                        device_checkin_consistency_token: checkin_consistency_token,
                        dfe_cookie,
                        dispenser_url: url.to_string(),
                        created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                        is_custom: false,
                    };

                    // Persist session to cache
                    let _ = Self::save_session(&session);
                    return Ok(session);
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!(
                        "Failed to connect to dispenser {}: {}",
                        url,
                        e
                    ));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All token dispensers failed to respond")))
    }

    /// Performs /checkin with Google servers to register device profile and obtain GSF ID
    async fn perform_checkin(
        client: &Client,
        device: &DeviceProfile,
    ) -> Result<(String, Option<String>)> {
        let checkin_body = Self::build_checkin_request_proto(device);
        let ua = format!("GoogleAuth/1.4 ({} {})", device.device, device.id);

        let res = client
            .post("https://android.clients.google.com/checkin")
            .header("app", "com.google.android.gms")
            .header("User-Agent", ua)
            .header("Content-Type", "application/x-protobuffer")
            .body(checkin_body)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Checkin failed: HTTP {}", res.status()));
        }

        let bytes = res.bytes().await?;
        let (android_id, consistency_token) = Self::parse_checkin_response(&bytes)?;
        let gsf_id = format!("{:016x}", android_id);

        Ok((gsf_id, consistency_token))
    }

    /// Formats Authorization header dynamically based on token type
    pub fn format_auth_header(token: &str) -> String {
        let clean = token.trim();
        if clean.starts_with("GoogleLogin ") || clean.starts_with("Bearer ") {
            clean.to_string()
        } else if clean.starts_with("oauth2_4/") || clean.starts_with("ya29.") {
            format!("Bearer {}", clean)
        } else {
            let val = clean.strip_prefix("Auth=").unwrap_or(clean);
            format!("GoogleLogin auth={}", val)
        }
    }

    /// Uploads DeviceConfigurationProto to /fdfe/uploadDeviceConfig
    async fn perform_upload_device_config(
        client: &Client,
        auth_token: &str,
        gsf_id: &str,
        user_agent: &str,
        consistency_token: Option<&str>,
        device: &DeviceProfile,
    ) -> Result<Option<String>> {
        let upload_body = Self::build_upload_device_config_proto(device);
        let auth_hdr = Self::format_auth_header(auth_token);
        let mut req = client
            .post("https://android.clients.google.com/fdfe/uploadDeviceConfig")
            .header(reqwest::header::AUTHORIZATION, auth_hdr)
            .header(reqwest::header::USER_AGENT, user_agent)
            .header("X-DFE-Device-Id", gsf_id)
            .header("X-DFE-Client-Id", "am-android-google")
            .header("X-DFE-Network-Type", "4")
            .header("Accept-Language", "en-US")
            .header("Content-Type", "application/x-protobuf")
            .body(upload_body);

        if let Some(token) = consistency_token {
            req = req.header("X-DFE-Device-Checkin-Consistency-Token", token);
        }

        let res = req.send().await?;
        if !res.status().is_success() {
            return Ok(None);
        }

        let bytes = res.bytes().await?;
        Ok(Self::find_string_at_tag(&bytes, 1))
    }

    /// Fetches /fdfe/toc and automatically accepts TOS if required
    async fn perform_toc_and_accept_tos(
        client: &Client,
        auth_token: &str,
        gsf_id: &str,
        user_agent: &str,
        config_token: Option<&str>,
        consistency_token: Option<&str>,
    ) -> Result<Option<String>> {
        let auth_hdr = Self::format_auth_header(auth_token);
        let mut req = client
            .get("https://android.clients.google.com/fdfe/toc")
            .header(reqwest::header::AUTHORIZATION, auth_hdr)
            .header(reqwest::header::USER_AGENT, user_agent)
            .header("X-DFE-Device-Id", gsf_id)
            .header("X-DFE-Client-Id", "am-android-google")
            .header("X-DFE-Network-Type", "4")
            .header("Accept-Language", "en-US");

        if let Some(cfg) = config_token {
            req = req.header("X-DFE-Device-Config-Token", cfg);
        }
        if let Some(chk) = consistency_token {
            req = req.header("X-DFE-Device-Checkin-Consistency-Token", chk);
        }

        let res = req.send().await?;
        if !res.status().is_success() {
            return Ok(None);
        }

        let bytes = res.bytes().await?;
        // Find TocResponse (tag 6 inside payload tag 1)
        let (tos_token, cookie) = Self::parse_toc_response(&bytes);

        if let Some(ref token) = tos_token {
            // Accept TOS
            let form_data = [("tost", token.as_str()), ("toscme", "false")];
            let mut accept_req = client
                .post("https://android.clients.google.com/fdfe/acceptTos")
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", auth_token),
                )
                .header(reqwest::header::USER_AGENT, user_agent)
                .header("X-DFE-Device-Id", gsf_id)
                .header("X-DFE-Client-Id", "am-android-google")
                .header("X-DFE-Network-Type", "4")
                .header("Accept-Language", "en-US")
                .form(&form_data);

            if let Some(cfg) = config_token {
                accept_req = accept_req.header("X-DFE-Device-Config-Token", cfg);
            }
            if let Some(chk) = consistency_token {
                accept_req = accept_req.header("X-DFE-Device-Checkin-Consistency-Token", chk);
            }

            let _ = accept_req.send().await;
        }

        Ok(cookie)
    }

    // Helper functions for protobuf encoding and parsing
    fn write_varint(buf: &mut Vec<u8>, mut val: u64) {
        loop {
            let byte = (val & 0x7F) as u8;
            val >>= 7;
            if val == 0 {
                buf.push(byte);
                break;
            } else {
                buf.push(byte | 0x80);
            }
        }
    }

    fn write_tag(buf: &mut Vec<u8>, field: u32, wire: u8) {
        Self::write_varint(buf, ((field as u64) << 3) | (wire as u64));
    }

    fn write_varint_field(buf: &mut Vec<u8>, field: u32, val: u64) {
        Self::write_tag(buf, field, 0);
        Self::write_varint(buf, val);
    }

    fn write_string_field(buf: &mut Vec<u8>, field: u32, s: &str) {
        Self::write_tag(buf, field, 2);
        Self::write_varint(buf, s.len() as u64);
        buf.extend_from_slice(s.as_bytes());
    }

    fn write_msg_field(buf: &mut Vec<u8>, field: u32, inner: &[u8]) {
        Self::write_tag(buf, field, 2);
        Self::write_varint(buf, inner.len() as u64);
        buf.extend_from_slice(inner);
    }

    fn build_device_config_proto(device: &DeviceProfile) -> Vec<u8> {
        let mut buf = Vec::new();
        Self::write_varint_field(&mut buf, 1, 3); // touchScreen
        Self::write_varint_field(&mut buf, 2, 1); // keyboard
        Self::write_varint_field(&mut buf, 3, 1); // navigation
        Self::write_varint_field(&mut buf, 4, 2); // screenLayout
        Self::write_varint_field(&mut buf, 5, 0); // hasHardKeyboard
        Self::write_varint_field(&mut buf, 6, 0); // hasFiveWayNavigation
        Self::write_varint_field(&mut buf, 7, device.density as u64);
        Self::write_varint_field(&mut buf, 8, device.gl_version as u64);

        for lib in &device.shared_libraries {
            Self::write_string_field(&mut buf, 9, lib);
        }
        for f in &device.features {
            Self::write_string_field(&mut buf, 10, f);
        }
        for p in &device.platforms {
            Self::write_string_field(&mut buf, 11, p);
        }
        Self::write_varint_field(&mut buf, 12, device.width as u64);
        Self::write_varint_field(&mut buf, 13, device.height as u64);

        for loc in &device.locales {
            Self::write_string_field(&mut buf, 14, loc);
        }
        for gl in &device.gl_extensions {
            Self::write_string_field(&mut buf, 15, gl);
        }

        Self::write_varint_field(&mut buf, 16, 0); // deviceClass
        Self::write_varint_field(&mut buf, 17, 50); // maxApkDownloadSizeMb
        Self::write_varint_field(&mut buf, 19, 0); // lowRamDevice
        Self::write_varint_field(&mut buf, 20, 8354971648); // totalMemoryBytes
        Self::write_varint_field(&mut buf, 21, 8); // maxNumOf_CPUCores

        for f in &device.features {
            let mut df = Vec::new();
            Self::write_string_field(&mut df, 1, f);
            Self::write_varint_field(&mut df, 2, 0);
            Self::write_msg_field(&mut buf, 26, &df);
        }

        buf
    }

    fn build_checkin_request_proto(device: &DeviceProfile) -> Vec<u8> {
        let dev_cfg = Self::build_device_config_proto(device);
        let now_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Build AndroidBuildProto
        let mut build_proto = Vec::new();
        Self::write_string_field(&mut build_proto, 1, &device.fingerprint);
        Self::write_string_field(&mut build_proto, 2, &device.hardware);
        Self::write_string_field(&mut build_proto, 3, &device.brand);
        Self::write_string_field(&mut build_proto, 4, &device.radio);
        Self::write_string_field(&mut build_proto, 5, &device.bootloader);
        Self::write_string_field(&mut build_proto, 6, &device.client);
        Self::write_varint_field(&mut build_proto, 7, now_sec);
        Self::write_varint_field(&mut build_proto, 8, device.gsf_version);
        Self::write_string_field(&mut build_proto, 9, &device.device);
        Self::write_varint_field(&mut build_proto, 10, device.sdk_int as u64);
        Self::write_string_field(&mut build_proto, 11, &device.model);
        Self::write_string_field(&mut build_proto, 12, &device.manufacturer);
        Self::write_string_field(&mut build_proto, 13, &device.product);
        Self::write_varint_field(&mut build_proto, 14, 0);

        // Build AndroidCheckinProto
        let mut checkin_proto = Vec::new();
        Self::write_msg_field(&mut checkin_proto, 1, &build_proto);
        Self::write_varint_field(&mut checkin_proto, 2, 0);
        Self::write_string_field(&mut checkin_proto, 6, &device.cell_operator);
        Self::write_string_field(&mut checkin_proto, 7, &device.sim_operator);
        Self::write_string_field(&mut checkin_proto, 8, &device.roaming);
        Self::write_varint_field(&mut checkin_proto, 9, 0);

        // AndroidCheckinRequest
        let mut root = Vec::new();
        Self::write_varint_field(&mut root, 2, 0);
        Self::write_msg_field(&mut root, 4, &checkin_proto);
        Self::write_string_field(&mut root, 6, "en_US");
        Self::write_string_field(&mut root, 12, &device.timezone);
        Self::write_varint_field(&mut root, 14, 3);
        Self::write_msg_field(&mut root, 18, &dev_cfg);
        Self::write_varint_field(&mut root, 20, 0);

        root
    }

    fn build_upload_device_config_proto(device: &DeviceProfile) -> Vec<u8> {
        let dev_cfg = Self::build_device_config_proto(device);
        let mut root = Vec::new();
        Self::write_msg_field(&mut root, 1, &dev_cfg);
        root
    }

    fn parse_varint(data: &[u8], mut pos: usize) -> Option<(u64, usize)> {
        let mut val = 0u64;
        let mut shift = 0;
        while pos < data.len() {
            let b = data[pos];
            pos += 1;
            val |= ((b & 0x7F) as u64) << shift;
            shift += 7;
            if (b & 0x80) == 0 {
                return Some((val, pos));
            }
        }
        None
    }

    fn parse_checkin_response(data: &[u8]) -> Result<(u64, Option<String>)> {
        let mut pos = 0;
        let mut android_id = 0u64;
        let mut consistency_token = None;

        while pos < data.len() {
            let (tag_wire, new_pos) = match Self::parse_varint(data, pos) {
                Some(p) => p,
                None => break,
            };
            pos = new_pos;
            let field = tag_wire >> 3;
            let wire = (tag_wire & 0x7) as u8;

            if wire == 0 {
                let (_, new_pos) = match Self::parse_varint(data, pos) {
                    Some(p) => p,
                    None => break,
                };
                pos = new_pos;
            } else if wire == 1 {
                // 64-bit fixed
                if pos + 8 <= data.len() {
                    let val_bytes: [u8; 8] = data[pos..pos + 8].try_into().unwrap_or_default();
                    let val = u64::from_le_bytes(val_bytes);
                    pos += 8;
                    if field == 7 {
                        android_id = val;
                    }
                } else {
                    break;
                }
            } else if wire == 2 {
                // Length-delimited
                let (len, new_pos) = match Self::parse_varint(data, pos) {
                    Some(p) => p,
                    None => break,
                };
                pos = new_pos;
                let end = pos + len as usize;
                if end <= data.len() {
                    let bytes = &data[pos..end];
                    pos = end;
                    if field == 12 {
                        consistency_token = String::from_utf8(bytes.to_vec()).ok();
                    }
                } else {
                    break;
                }
            } else if wire == 5 {
                pos += 4;
            } else {
                break;
            }
        }

        if android_id == 0 {
            return Err(anyhow::anyhow!("Checkin response missing Android ID"));
        }

        Ok((android_id, consistency_token))
    }

    fn find_string_at_tag(data: &[u8], target_field: u32) -> Option<String> {
        let mut pos = 0;
        while pos < data.len() {
            let (tag_wire, new_pos) = Self::parse_varint(data, pos)?;
            pos = new_pos;
            let field = tag_wire >> 3;
            let wire = (tag_wire & 0x7) as u8;

            if wire == 0 {
                let (_, new_pos) = Self::parse_varint(data, pos)?;
                pos = new_pos;
            } else if wire == 2 {
                let (len, new_pos) = Self::parse_varint(data, pos)?;
                pos = new_pos;
                let end = pos + len as usize;
                if end <= data.len() {
                    let slice = &data[pos..end];
                    pos = end;
                    if field == target_field as u64 {
                        if let Ok(s) = String::from_utf8(slice.to_vec()) {
                            return Some(s);
                        }
                    }
                    if let Some(s) = Self::find_string_at_tag(slice, target_field) {
                        return Some(s);
                    }
                } else {
                    break;
                }
            } else if wire == 5 {
                pos += 4;
            } else {
                break;
            }
        }
        None
    }

    fn parse_toc_response(data: &[u8]) -> (Option<String>, Option<String>) {
        let mut tos_token = None;
        let mut cookie = None;

        // Traverse down payload (tag 1) -> TocResponse (tag 6)
        let mut pos = 0;
        while pos < data.len() {
            let (tag_wire, new_pos) = match Self::parse_varint(data, pos) {
                Some(p) => p,
                None => break,
            };
            pos = new_pos;
            let field = tag_wire >> 3;
            let wire = (tag_wire & 0x7) as u8;

            if wire == 0 {
                let (_, new_pos) = match Self::parse_varint(data, pos) {
                    Some(p) => p,
                    None => break,
                };
                pos = new_pos;
            } else if wire == 2 {
                let (len, new_pos) = match Self::parse_varint(data, pos) {
                    Some(p) => p,
                    None => break,
                };
                pos = new_pos;
                let end = pos + len as usize;
                if end <= data.len() {
                    let slice = &data[pos..end];
                    pos = end;
                    if field == 1 || field == 6 {
                        let (t, c) = Self::parse_toc_fields(slice);
                        if t.is_some() {
                            tos_token = t;
                        }
                        if c.is_some() {
                            cookie = c;
                        }
                    }
                } else {
                    break;
                }
            } else if wire == 5 {
                pos += 4;
            } else {
                break;
            }
        }

        (tos_token, cookie)
    }

    fn parse_toc_fields(data: &[u8]) -> (Option<String>, Option<String>) {
        let mut tos_token = None;
        let mut cookie = None;
        let mut pos = 0;

        while pos < data.len() {
            let (tag_wire, new_pos) = match Self::parse_varint(data, pos) {
                Some(p) => p,
                None => break,
            };
            pos = new_pos;
            let field = tag_wire >> 3;
            let wire = (tag_wire & 0x7) as u8;

            if wire == 0 {
                let (_, new_pos) = match Self::parse_varint(data, pos) {
                    Some(p) => p,
                    None => break,
                };
                pos = new_pos;
            } else if wire == 2 {
                let (len, new_pos) = match Self::parse_varint(data, pos) {
                    Some(p) => p,
                    None => break,
                };
                pos = new_pos;
                let end = pos + len as usize;
                if end <= data.len() {
                    let slice = &data[pos..end];
                    pos = end;
                    if field == 7 {
                        tos_token = String::from_utf8(slice.to_vec()).ok();
                    } else if field == 22 {
                        cookie = String::from_utf8(slice.to_vec()).ok();
                    }
                } else {
                    break;
                }
            } else if wire == 5 {
                pos += 4;
            } else {
                break;
            }
        }

        (tos_token, cookie)
    }
}
