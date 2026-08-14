use crate::auth::AuthManager;
use crate::models::{
    AppDelivery, AppInfo, AuthSession, DownloadFileType, PlayDownloadFile, VersionReport,
};
use crate::proto::{ProtoMessage, ProtoValue};
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

const GOOGLE_PLAY_BASE_URL: &str = "https://android.clients.google.com/fdfe";
const EXODUS_SEARCH_URL: &str = "https://reports.exodus-privacy.eu.org/api/search";
const DEFAULT_EXODUS_API_KEY: &str = "bbe6ebae4ad45a9cbacb17d69739799b8df2c7ae";

#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub version_code: u64,
    pub version_name: String,
}

pub struct GooglePlayApi {
    client: Client,
    auth_manager: AuthManager,
}

impl GooglePlayApi {
    pub fn new(auth_manager: AuthManager) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .unwrap_or_default();
        Self {
            client,
            auth_manager,
        }
    }

    fn build_headers(session: &AuthSession) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&session.user_agent)?);
        let auth_header_val = if session.auth_token.starts_with("GoogleLogin ")
            || session.auth_token.starts_with("Bearer ")
        {
            session.auth_token.clone()
        } else if session.auth_token.starts_with("oauth2_4/")
            || session.auth_token.starts_with("ya29.")
        {
            format!("Bearer {}", session.auth_token)
        } else {
            let clean = session
                .auth_token
                .strip_prefix("Auth=")
                .unwrap_or(&session.auth_token);
            format!("GoogleLogin auth={}", clean)
        };

        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_header_val)?);
        headers.insert("X-DFE-Device-Id", HeaderValue::from_str(&session.gsf_id)?);
        headers.insert(
            "X-DFE-Client-Id",
            HeaderValue::from_static("am-android-google"),
        );
        headers.insert("Accept-Language", HeaderValue::from_static("en-US"));
        headers.insert(
            "X-DFE-Encoded-Targets",
            HeaderValue::from_static(
                "CAESN/qigQYC2AMBFfUbyA7SM5Ij/CvfBoIDgxHqGP8R3xzIBvoQtBKFDZ4HAY4FrwSVMasHBO0O2Q8akgYRAQECAQO7AQEpKZ0CnwECAwRrAQYBr9PPAoK7sQMBAQMCBAkIDAgBAwEDBAICBAUZEgMEBAMLAQEBBQEBAcYBARYED+cBfS8CHQEKkAEMMxcBIQoUDwYHIjd3DQ4MFk0JWGYZEREYAQOLAYEBFDMIEYMBAgICAgICOxkCD18LGQKEAcgDBIQBAgGLARkYCy8oBTJlBCUocxQn0QUBDkkGxgNZQq0BZSbeAmIDgAEBOgGtAaMCDAOQAZ4BBIEBKUtQUYYBQscDDxPSARA1oAEHAWmnAsMB2wFyywGLAxol+wImlwOOA80CtwN26A0WjwJVbQEJPAH+BRDeAfkHK/ABASEBCSAaHQemAzkaRiu2Ad8BdXeiAwEBGBUBBN4LEIABK4gB2AFLfwECAdoENq0CkQGMBsIBiQEtiwGgA1zyAUQ4uwS8AwhsvgPyAcEDF27vApsBHaICGhl3GSKxAR8MC6cBAgItmQYG9QIeywLvAeYBDArLAh8HASI4ELICDVmVBgsY/gHWARtcAsMBpALiAdsBA7QBpAJmIArpByn0AyAKBwHTARIHAX8D+AMBcRIBBbEDmwUBMacCHAciNp0BAQF0OgQLJDuSAh54kwFSP0eeAQQ4M5EBQgMEmwFXywFo0gFyWwMcapQBBugBPUW2AVgBKmy3AR6PAbMBGQxrUJECvQR+8gFoWDsYgQNwRSczBRXQAgtRswEW0ALMAREYAUEBIG6yATYCRE8OxgER8gMBvQEDRkwLc8MBTwHZAUOnAXiiBakDIbYBNNcCIUmuArIBSakBrgFHKs0EgwV/G3AD0wE6LgECtQJ4xQFwFbUCjQPkBS6vAQqEAUZF3QIM9wEhCoYCQhXsBCyZArQDugIziALWAdIBlQHwBdUErQE6qQaSA4EEIvYBHir9AQVLmgMCApsCKAwHuwgrENsBAjNYswEVmgIt7QJnN4wDEnta+wGfAcUBxgEtEFXQAQWdAUAeBcwBAQM7rAEJATJ0LENrdh73A6UBhAE+qwEeASxLZUMhDREuH0CGARbd7K0GlQo",
            ),
        );
        headers.insert(
            "X-DFE-Phenotype",
            HeaderValue::from_static(
                "H4sIAAAAAAAAAB3OO3KjMAAA0KRNuWXukBkBQkAJ2MhgAZb5u2GCwQZbCH_EJ77QHmgvtDtbv-Z9_H63zXXU0NVPB1odlyGy7751Q3CitlPDvFd8lxhz3tpNmz7P92CFw73zdHU2Ie0Ad2kmR8lxhiErTFLt3RPGfJQHSDy7Clw10bg8kqf2owLokN4SecJTLoSwBnzQSd652_MOf2d1vKBNVedzg4ciPoLz2mQ8efGAgYeLou-l-PXn_7Sna1MfhHuySxt-4esulEDp8Sbq54CPPKjpANW-lkU2IZ0F92LBI-ukCKSptqeq1eXU96LD9nZfhKHdtjSWwJqUm_2r6pMHOxk01saVanmNopjX3YxQafC4iC6T55aRbC8nTI98AF_kItIQAJb5EQxnKTO7TZDWnr01HVPxelb9A2OWX6poidMWl16K54kcu_jhXw-JSBQkVcD_fPsLSZu6joIBAAA",
            ),
        );
        headers.insert("X-DFE-Network-Type", HeaderValue::from_static("4"));
        headers.insert(
            "X-DFE-Request-Params",
            HeaderValue::from_static("timeoutMs=4000"),
        );
        headers.insert("X-DFE-MCCMNC", HeaderValue::from_static("31038"));
        headers.insert("X-DFE-UserLanguages", HeaderValue::from_static("en_US"));
        headers.insert(
            "X-Limit-Ad-Tracking-Enabled",
            HeaderValue::from_static("false"),
        );

        if let Some(ref cfg_token) = session.device_config_token {
            if !cfg_token.is_empty() {
                headers.insert(
                    "X-DFE-Device-Config-Token",
                    HeaderValue::from_str(cfg_token)?,
                );
            }
        }
        if let Some(ref chk_token) = session.device_checkin_consistency_token {
            if !chk_token.is_empty() {
                headers.insert(
                    "X-DFE-Device-Checkin-Consistency-Token",
                    HeaderValue::from_str(chk_token)?,
                );
            }
        }
        if let Some(ref dfe_cookie) = session.dfe_cookie {
            if !dfe_cookie.is_empty() {
                headers.insert("X-DFE-Cookie", HeaderValue::from_str(dfe_cookie)?);
            }
        }

        Ok(headers)
    }

    /// Search for apps on Google Play Store with smart multi-source resolution
    pub async fn search(&self, query: &str) -> Result<Vec<AppInfo>> {
        // 1. If query looks like a package name (contains dot and no spaces), fetch details directly
        if query.contains('.') && !query.contains(' ') {
            if let Ok(direct_info) = self.details(query).await {
                let mut results = vec![direct_info];
                if let Ok(more) = self.search_web_scraper(query).await {
                    for app in more {
                        if app.package_name != query {
                            results.push(app);
                        }
                    }
                }
                return Ok(results);
            }
        }

        // 2. Try rich web search scraper (matches AuroraStore WebSearchHelper)
        if let Ok(web_results) = self.search_web_scraper(query).await {
            if !web_results.is_empty() {
                return Ok(web_results);
            }
        }

        // 3. Fallback to Google Play mobile API /fdfe/search
        let mut session = self.auth_manager.get_session(false).await?;
        let mut headers = Self::build_headers(&session)?;

        let url = format!(
            "{}/search?c=3&q={}",
            GOOGLE_PLAY_BASE_URL,
            urlencoding::encode(query)
        );
        let mut res = self
            .client
            .get(&url)
            .headers(headers.clone())
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::UNAUTHORIZED
            || res.status() == reqwest::StatusCode::FORBIDDEN
        {
            session = self.auth_manager.get_session(true).await?;
            headers = Self::build_headers(&session)?;
            res = self.client.get(&url).headers(headers).send().await?;
        }

        if res.status().is_success() {
            let bytes = res.bytes().await?;
            let apps = self.parse_search_response(&bytes);
            if !apps.is_empty() {
                return Ok(apps);
            }
        }

        Ok(Vec::new())
    }

    /// Search parser for Google Play web search results (extracts accurate titles, devs, ratings)
    async fn search_web_scraper(&self, query: &str) -> Result<Vec<AppInfo>> {
        let url = format!(
            "https://play.google.com/store/search?q={}&c=apps&hl=en&gl=US",
            urlencoding::encode(query)
        );
        let resp = self
            .client
            .get(&url)
            .header(
                USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
            )
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await?;

        let html = resp.text().await?;
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let parts: Vec<&str> = html.split("href=\"/store/apps/details?id=").collect();

        for part in parts.iter().skip(1) {
            let end_pkg = match part.find('"') {
                Some(p) => p,
                None => continue,
            };
            let pkg = &part[..end_pkg];
            if pkg.is_empty() || !pkg.contains('.') || !seen.insert(pkg.to_string()) {
                continue;
            }

            let chunk_len = std::cmp::min(part.len(), 2000);
            let chunk = &part[..chunk_len];

            // Title: check DdYX5 or vWM94c
            let mut title = pkg.to_string();
            if let Some(pos) = chunk.find("class=\"DdYX5\">") {
                let rest = &chunk[pos + 14..];
                if let Some(end) = rest.find('<') {
                    title = unescape_html(&rest[..end]);
                }
            } else if let Some(pos) = chunk.find("class=\"vWM94c\">") {
                let rest = &chunk[pos + 15..];
                if let Some(end) = rest.find('<') {
                    title = unescape_html(&rest[..end]);
                }
            }

            // Developer: check wMUdtb, LbQbAe, or w2kbKc
            let mut dev = "Unknown".to_string();
            if let Some(pos) = chunk.find("class=\"wMUdtb\">") {
                let rest = &chunk[pos + 15..];
                if let Some(end) = rest.find('<') {
                    dev = unescape_html(&rest[..end]);
                }
            } else if let Some(pos) = chunk.find("class=\"LbQbAe\">") {
                let rest = &chunk[pos + 15..];
                if let Some(end) = rest.find('<') {
                    dev = unescape_html(&rest[..end]);
                }
            } else if let Some(pos) = chunk.find("class=\"w2kbKc\">") {
                let rest = &chunk[pos + 15..];
                if let Some(end) = rest.find('<') {
                    dev = unescape_html(&rest[..end]);
                }
            }

            // Rating: check aria-label="Rated X.X stars"
            let mut rating = 4.5;
            if let Some(pos) = chunk.find("aria-label=\"Rated ") {
                let rest = &chunk[pos + 18..];
                let num_str: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(r) = num_str.parse::<f32>() {
                    rating = r;
                }
            }

            // Downloads: check ClM7O or wVqUob
            let mut downloads_text = "N/A".to_string();
            if let Some(pos) = chunk.find("class=\"ClM7O\">") {
                let rest = &chunk[pos + 14..];
                if let Some(end) = rest.find('<') {
                    downloads_text = unescape_html(&rest[..end]);
                }
            }

            results.push(AppInfo {
                package_name: pkg.to_string(),
                title,
                developer: dev,
                version_name: "Latest".to_string(),
                version_code: 0,
                size_bytes: 0,
                category: "Application".to_string(),
                rating,
                downloads_text,
                icon_url: None,
                description: None,
            });

            if results.len() >= 25 {
                break;
            }
        }

        Ok(results)
    }
}

fn unescape_html(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

impl GooglePlayApi {
    fn parse_search_response(&self, data: &[u8]) -> Vec<AppInfo> {
        let mut results = Vec::new();
        let top = ProtoMessage::parse(data);

        let payload = match top.get_message(1) {
            Some(p) => p,
            None => return results,
        };

        self.extract_docs_from_proto(&payload, &mut results);
        results
    }

    fn extract_docs_from_proto(&self, msg: &ProtoMessage, results: &mut Vec<AppInfo>) {
        for field in &msg.fields {
            if let ProtoValue::LengthDelimited(bytes) = &field.value {
                let sub = ProtoMessage::parse(bytes);
                if let (Some(pkg), Some(title)) = (sub.get_string(1), sub.get_string(5)) {
                    if pkg.contains('.') && !title.is_empty() {
                        let dev = sub.get_string(6).unwrap_or_else(|| "Unknown".to_string());
                        let app_details = sub.get_message(14).unwrap_or_default();
                        let version_code = app_details.get_varint(3).unwrap_or(0);
                        let version_name = app_details
                            .get_string(4)
                            .unwrap_or_else(|| "Latest".to_string());
                        let size_bytes = app_details.get_varint(8).unwrap_or(0);

                        results.push(AppInfo {
                            package_name: pkg,
                            title,
                            developer: dev,
                            version_name,
                            version_code,
                            size_bytes,
                            category: "Application".to_string(),
                            rating: 4.5,
                            downloads_text: "1M+".to_string(),
                            icon_url: None,
                            description: None,
                        });
                    }
                }
                self.extract_docs_from_proto(&sub, results);
            }
        }
    }

    /// Fetch app details with smart auto-token-refresh
    pub async fn details(&self, package_name: &str) -> Result<AppInfo> {
        let mut session = self.auth_manager.get_session(false).await?;
        let mut headers = Self::build_headers(&session)?;

        let url = format!("{}/details?doc={}", GOOGLE_PLAY_BASE_URL, package_name);
        let mut res = self
            .client
            .get(&url)
            .headers(headers.clone())
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::UNAUTHORIZED
            || res.status() == reqwest::StatusCode::FORBIDDEN
        {
            session = self.auth_manager.get_session(true).await?;
            headers = Self::build_headers(&session)?;
            res = self.client.get(&url).headers(headers).send().await?;
        }

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get details for {}: HTTP {}",
                package_name,
                res.status()
            ));
        }

        let bytes = res.bytes().await?;
        self.parse_details_response(package_name, &bytes)
    }

    fn parse_details_response(&self, package_name: &str, data: &[u8]) -> Result<AppInfo> {
        let top = ProtoMessage::parse(data);
        let payload = top
            .get_message(1)
            .context("Invalid protobuf: missing payload")?;
        let tag2 = payload
            .get_message(2)
            .context("Missing docDetailsResponse")?;
        let doc = tag2
            .get_message(4)
            .context("Missing DocV2 in details response")?;

        let title = doc
            .get_string(5)
            .unwrap_or_else(|| package_name.to_string());
        let developer = doc.get_string(6).unwrap_or_else(|| "Unknown".to_string());
        let description = doc.get_string(27);

        let mut _offer_type = 1;
        for field in &doc.fields {
            if field.tag == 8 {
                if let ProtoValue::LengthDelimited(bytes) = &field.value {
                    let offer = ProtoMessage::parse(bytes);
                    if let Some(ot) = offer.get_varint(1) {
                        _offer_type = ot as u32;
                    }
                }
            }
        }

        let mut version_code = 0;
        let mut version_name = "Latest".to_string();
        let mut size_bytes = 0;

        fn find_app_details(msg: &ProtoMessage, path: &str) -> (u64, Option<String>, u64) {
            let vc = 0;
            let vn = None;
            let sz = 0;

            // AppDetails proto typically has tag 3 = versionCode, tag 4 = versionString, tag 8 = size
            if let (Some(code), Some(name)) = (msg.get_varint(3), msg.get_string(4)) {
                if !name.is_empty() && code > 0 {
                    return (code, Some(name), msg.get_varint(8).unwrap_or(0));
                }
            }

            for f in &msg.fields {
                if let ProtoValue::LengthDelimited(bytes) = &f.value {
                    let sub = ProtoMessage::parse(bytes);
                    let (sub_vc, sub_vn, sub_sz) =
                        find_app_details(&sub, &format!("{}.{}", path, f.tag));
                    if sub_vc > 0 && sub_vn.is_some() {
                        return (sub_vc, sub_vn, sub_sz);
                    }
                }
            }
            (vc, vn, sz)
        }

        let (found_vc, found_vn, found_sz) = find_app_details(&doc, "doc");
        if found_vc > 0 {
            version_code = found_vc;
        }
        if let Some(v) = found_vn {
            version_name = v;
        }
        if found_sz > 0 {
            size_bytes = found_sz;
        }

        let mut downloads_text = "N/A".to_string();
        if let Some(metrics_outer) = doc.get_message(13) {
            if let Some(metrics_inner) = metrics_outer.get_message(1) {
                if let Some(dl) = metrics_inner.get_string(61) {
                    downloads_text = dl;
                } else if let Some(dl) = metrics_inner.get_string(13) {
                    downloads_text = dl;
                }
            }
        }

        let mut rating = 4.5;
        if let Some(rating_msg) = doc.get_message(14) {
            if let Some(r_str) = rating_msg.get_string(17) {
                if let Ok(r) = r_str.parse::<f32>() {
                    rating = r;
                }
            }
        }

        let mut category = "Application".to_string();
        if let Some(metrics_outer) = doc.get_message(13) {
            if let Some(m1) = metrics_outer.get_message(1) {
                if let Some(m66) = m1.get_message(66) {
                    if let Some(m9) = m66.get_message(9) {
                        if let Some(m1_sub) = m9.get_message(1) {
                            if let Some(cat) = m1_sub.get_string(1) {
                                category = cat;
                            }
                        }
                    }
                }
            }
        }

        Ok(AppInfo {
            package_name: package_name.to_string(),
            title,
            developer,
            version_name,
            version_code,
            size_bytes,
            category,
            rating,
            downloads_text,
            icon_url: None,
            description,
        })
    }

    /// Resolves user-provided version string (version name like "8.22.2", "v8.22.2", versionCode like "173301", or "latest")
    /// to the corresponding integer versionCode. If the requested version is not found, suggests available versions found.
    pub async fn resolve_version(
        &self,
        package_name: &str,
        requested_version: Option<&str>,
    ) -> Result<ResolvedVersion> {
        let details = self.details(package_name).await?;

        let req = match requested_version {
            Some(v) if !v.trim().is_empty() && !v.eq_ignore_ascii_case("latest") => v.trim(),
            _ => {
                return Ok(ResolvedVersion {
                    version_code: details.version_code,
                    version_name: details.version_name,
                });
            }
        };

        // 1. Check if user passed an exact integer versionCode
        if let Ok(code) = req.parse::<u64>() {
            let name = if code == details.version_code {
                details.version_name.clone()
            } else {
                format!("{}", code)
            };
            return Ok(ResolvedVersion {
                version_code: code,
                version_name: name,
            });
        }

        // 2. Normalize and check against latest version name
        let clean_req = req.trim_start_matches(|c| c == 'v' || c == 'V');
        let clean_latest = details
            .version_name
            .trim_start_matches(|c| c == 'v' || c == 'V');

        if clean_latest.eq_ignore_ascii_case(clean_req) {
            return Ok(ResolvedVersion {
                version_code: details.version_code,
                version_name: details.version_name,
            });
        }

        // 3. Search historical versions in Exodus Privacy
        let exodus_api = ExodusApi::new();
        let history = exodus_api
            .fetch_versions(package_name)
            .await
            .unwrap_or_default();

        for report in &history {
            let clean_report_ver = report
                .version_name
                .trim_start_matches(|c| c == 'v' || c == 'V');
            if clean_report_ver.eq_ignore_ascii_case(clean_req)
                || clean_report_ver.starts_with(clean_req)
                || report.version_name.eq_ignore_ascii_case(req)
            {
                return Ok(ResolvedVersion {
                    version_code: report.version_code,
                    version_name: report.version_name.clone(),
                });
            }
        }

        // 4. If not found, format a helpful list of available suggestions
        let mut msg = format!(
            "Version '{}' was not found for '{}'.\n\n",
            req, package_name
        );

        let mut available = Vec::new();
        if details.version_code > 0 {
            available.push(format!(
                "  • {} (versionCode: {}) - Latest on Google Play",
                details.version_name, details.version_code
            ));
        }

        for report in history.iter().take(8) {
            let date_str = if !report.release_date.is_empty() {
                format!(" - Released: {}", report.release_date)
            } else {
                "".to_string()
            };
            available.push(format!(
                "  • {} (versionCode: {}){}",
                report.version_name, report.version_code, date_str
            ));
        }

        if !available.is_empty() {
            msg.push_str("Available versions found:\n");
            msg.push_str(&available.join("\n"));
            msg.push_str("\n\n💡 Run 'gplay versions ");
            msg.push_str(package_name);
            msg.push_str("' to view all historical versions.");
        }

        Err(anyhow::anyhow!(msg))
    }

    /// Calls /fdfe/acquire, /fdfe/purchase and /fdfe/delivery with smart auto-token-refresh
    pub async fn acquire_and_deliver(
        &self,
        package_name: &str,
        version_code: u64,
        offer_type: u32,
    ) -> Result<AppDelivery> {
        let mut session = self.auth_manager.get_session(false).await?;
        let mut headers = Self::build_headers(&session)?;

        // Step 0: POST /fdfe/acquire — grants the anonymous session a license.
        // This is required before /fdfe/purchase will return a valid deliveryToken.
        // Protobuf layout (manually encoded without prost):
        //   field 1 (package, msg):
        //     field 1 (payload, msg): field 3 (string) = packageName, field 1 (varint) = 1, field 2 (varint) = 3
        //     field 2 (varint) = 1
        //   field 2 (version, msg): field 1 (varint) = versionCode, field 3 (varint) = 0
        //   field 3 (offerType, varint) = offerType
        //   field 15 (varint) = 0
        //   field 25 (varint) = 2
        let acquire_body = Self::build_acquire_proto(package_name, version_code, offer_type);
        let acquire_url = format!("{}/acquire", GOOGLE_PLAY_BASE_URL);
        let mut acquire_headers = headers.clone();
        acquire_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-protobuf"),
        );
        let acq_resp = self
            .client
            .post(&acquire_url)
            .headers(acquire_headers)
            .body(acquire_body)
            .send()
            .await;
        if let Ok(ref r) = acq_resp {
            if r.status() == reqwest::StatusCode::UNAUTHORIZED
                || r.status() == reqwest::StatusCode::FORBIDDEN
            {
                session = self.auth_manager.get_session(true).await?;
                headers = Self::build_headers(&session)?;
            }
        }

        // Step 1: POST /fdfe/purchase — gets encodedDeliveryToken
        let purchase_url = format!("{}/purchase", GOOGLE_PLAY_BASE_URL);
        let form_body = format!("doc={}&ot={}&vc={}", package_name, offer_type, version_code);

        let mut purchase_headers = headers.clone();
        purchase_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
        );

        let mut purchase_resp = self
            .client
            .post(&purchase_url)
            .headers(purchase_headers.clone())
            .body(form_body.clone())
            .send()
            .await?;

        // Smart auto-refresh token if expired (only for anonymous sessions)
        if purchase_resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || purchase_resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            if session.is_custom {
                return Err(anyhow::anyhow!(
                    "Custom Google Account session for '{}' returned HTTP {}.\nPlease re-authenticate with 'gplay-cli auth login'.",
                    session.email,
                    purchase_resp.status()
                ));
            }
            session = self.auth_manager.get_session(true).await?;
            headers = Self::build_headers(&session)?;
            purchase_headers = headers.clone();
            purchase_headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
            );
            purchase_resp = self
                .client
                .post(&purchase_url)
                .headers(purchase_headers)
                .body(form_body)
                .send()
                .await?;
        }

        if !purchase_resp.status().is_success() {
            let status = purchase_resp.status();
            let text = purchase_resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Purchase failed for {} (vc={}): HTTP {} - {}",
                package_name,
                version_code,
                status,
                text
            ));
        }

        let purchase_bytes = purchase_resp.bytes().await?;
        let purchase_proto = ProtoMessage::parse(&purchase_bytes);

        let mut delivery_token = None;
        if let Some(payload) = purchase_proto.get_message(1) {
            delivery_token = Self::find_tag_string(&payload, 55);
        }

        // Step 2: GET /fdfe/delivery
        let mut delivery_url = format!(
            "{}/delivery?doc={}&ot={}&vc={}",
            GOOGLE_PLAY_BASE_URL, package_name, offer_type, version_code
        );
        if let Some(ref dtok) = delivery_token {
            delivery_url.push_str("&dtok=");
            delivery_url.push_str(&urlencoding::encode(dtok));
        }

        let mut delivery_resp = self
            .client
            .get(&delivery_url)
            .headers(headers.clone())
            .send()
            .await?;

        if delivery_resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || delivery_resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            if session.is_custom {
                return Err(anyhow::anyhow!(
                    "Custom Google Account session for '{}' returned HTTP {}.\nPlease re-authenticate with 'gplay-cli auth login'.",
                    session.email,
                    delivery_resp.status()
                ));
            }
            session = self.auth_manager.get_session(true).await?;
            headers = Self::build_headers(&session)?;
            delivery_resp = self
                .client
                .get(&delivery_url)
                .headers(headers)
                .send()
                .await?;
        }

        if !delivery_resp.status().is_success() {
            let status = delivery_resp.status();
            let text = delivery_resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Delivery request failed for {} (vc={}): HTTP {} - {}",
                package_name,
                version_code,
                status,
                text
            ));
        }

        let delivery_bytes = delivery_resp.bytes().await?;
        match self.parse_delivery_response(package_name, version_code, &delivery_bytes) {
            Ok(delivery) => Ok(delivery),
            Err(play_err) => {
                // If the user is on a custom Google account, do not overwrite with anonymous dispenser
                if session.is_custom {
                    return Err(play_err);
                }

                // Google Play returned a restricted/empty delivery (Status 2).
                // Retry with a freshly dispensed Aurora anonymous token.
                eprintln!("ℹ️  Google Play returned restricted delivery — refreshing Aurora session and retrying...");
                match self.auth_manager.get_session(true).await {
                    Ok(new_session) => {
                        let new_headers = Self::build_headers(&new_session)?;
                        let retry_resp = self
                            .client
                            .get(&delivery_url)
                            .headers(new_headers)
                            .send()
                            .await;
                        match retry_resp {
                            Ok(resp) if resp.status().is_success() => {
                                let retry_bytes = resp.bytes().await?;
                                self.parse_delivery_response(
                                    package_name,
                                    version_code,
                                    &retry_bytes,
                                )
                                .map_err(|_| play_err)
                            }
                            _ => Err(play_err),
                        }
                    }
                    Err(_) => Err(play_err),
                }
            }
        }
    }

    fn parse_delivery_response(
        &self,
        package_name: &str,
        version_code: u64,
        data: &[u8],
    ) -> Result<AppDelivery> {
        let top = ProtoMessage::parse(data);
        let payload = top
            .get_message(1)
            .context("Invalid delivery protobuf: missing payload")?;

        let delivery_resp = payload
            .get_message(21)
            .context("Missing deliveryResponse (tag 21)")?;

        let status = delivery_resp.get_varint(1).unwrap_or(0);
        let app_delivery_data = match delivery_resp.get_message(2) {
            Some(d) => d,
            None => {
                let advice = match status {
                    2 => "\n\n💡 Google Play reported AppNotSupported (Status 2). The public anonymous dispenser cannot license this package on this device architecture.\n👉 Try providing custom credentials with GPLAY_AUTH_TOKEN / GPLAY_EMAIL or downloading a different version code with 'gplay versions <package>'.",
                    3 => "\n\n💡 Google Play reported AppNotPurchased (Status 3). This app requires a purchase or license associated with a Google Account.",
                    4 => "\n\n💡 Google Play reported DeviceNotCompatible (Status 4). This app is incompatible with the current device profile.",
                    6 => "\n\n💡 Google Play reported GeoRestricted (Status 6). This app is not available in the current region/country.",
                    7 => "\n\n💡 Google Play reported AppRemoved (Status 7). This app has been removed from the Google Play Store.",
                    9 => "\n\n💡 Google Play reported AppNotSupported (Status 9). The app architecture is not supported.",
                    _ => "",
                };
                return Err(anyhow::anyhow!(
                    "Delivery failed for {} (vc={}): Delivery status {}{}",
                    package_name,
                    version_code,
                    status,
                    advice
                ));
            }
        };

        let download_url = app_delivery_data
            .get_string(3)
            .context("Missing primary downloadUrl in delivery data")?;
        let download_size = app_delivery_data.get_varint(1).unwrap_or(0);

        let mut base_apk = PlayDownloadFile {
            name: format!("{}_{}_base.apk", package_name, version_code),
            file_type: DownloadFileType::BaseApk,
            download_url,
            size_bytes: download_size,
            sha1: None,
            sha256: None,
        };

        if let Some(download_auth) = app_delivery_data.get_message(4) {
            base_apk.sha1 = download_auth.get_string(1);
        }

        let mut split_files = Vec::new();
        let mut total_size = download_size;

        for split_msg in app_delivery_data.get_messages(15) {
            let split_name = split_msg
                .get_string(1)
                .unwrap_or_else(|| "split_unknown".to_string());
            let split_size = split_msg.get_varint(2).unwrap_or(0);
            let split_url = match split_msg.get_string(5) {
                Some(u) => u,
                None => continue,
            };

            total_size += split_size;
            let filename = format!("{}_{}_{}.apk", package_name, version_code, split_name);

            split_files.push(PlayDownloadFile {
                name: filename,
                file_type: DownloadFileType::SplitConfig,
                download_url: split_url,
                size_bytes: split_size,
                sha1: None,
                sha256: None,
            });
        }

        Ok(AppDelivery {
            package_name: package_name.to_string(),
            version_code,
            base_file: base_apk,
            split_files,
            total_size,
        })
    }

    fn find_tag_string(msg: &ProtoMessage, target_tag: u32) -> Option<String> {
        if let Some(s) = msg.get_string(target_tag) {
            return Some(s);
        }
        for field in &msg.fields {
            if let ProtoValue::LengthDelimited(bytes) = &field.value {
                let sub = ProtoMessage::parse(bytes);
                if let Some(s) = Self::find_tag_string(&sub, target_tag) {
                    return Some(s);
                }
            }
        }
        None
    }

    /// Manually encodes the AcquireRequest protobuf for POST /fdfe/acquire.
    /// Field structure matches com.aurora.gplayapi.AcquireRequest:
    /// Constructs binary protobuf payload for POST /fdfe/acquire (AcquireRequest)
    /// Wire structure matching AcquireApp.proto:
    ///   tag 1 (package msg):
    ///     tag 1 (payload msg): tag 1 = packageName (string), tag 2 = 1 (varint), tag 3 = 3 (varint)
    ///     tag 2 = 1 (varint)
    ///   tag 12 (version msg): tag 1 = versionCode (varint), tag 3 = 0 (varint)
    ///   tag 13 = offerType (varint)
    ///   tag 15 = 0 (varint)
    ///   tag 22 = nonce ("nonce=" + base64 random bytes)
    ///   tag 25 = 2 (varint)
    ///   tag 30 (m30 msg): tag 1 = 2 (varint), tag 2 = 0 (varint)
    fn build_acquire_proto(package_name: &str, version_code: u64, offer_type: u32) -> Vec<u8> {
        // Proto wire encoding helpers
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
            write_varint(buf, ((field as u64) << 3) | wire as u64);
        }
        fn write_string(buf: &mut Vec<u8>, field: u32, s: &str) {
            write_tag(buf, field, 2); // wire type 2 = length-delimited
            write_varint(buf, s.len() as u64);
            buf.extend_from_slice(s.as_bytes());
        }
        fn write_msg(buf: &mut Vec<u8>, field: u32, inner: Vec<u8>) {
            write_tag(buf, field, 2);
            write_varint(buf, inner.len() as u64);
            buf.extend_from_slice(&inner);
        }
        fn write_varint_field(buf: &mut Vec<u8>, field: u32, val: u64) {
            write_tag(buf, field, 0); // wire type 0 = varint
            write_varint(buf, val);
        }

        // Build inner payload message (field 1 of Package)
        let mut payload_msg: Vec<u8> = Vec::new();
        write_string(&mut payload_msg, 1, package_name);
        write_varint_field(&mut payload_msg, 2, 1);
        write_varint_field(&mut payload_msg, 3, 3);

        // Build Package message (field 1 of AcquireRequest)
        let mut pkg_msg: Vec<u8> = Vec::new();
        write_msg(&mut pkg_msg, 1, payload_msg);
        write_varint_field(&mut pkg_msg, 2, 1);

        // Build Version message (field 12 of AcquireRequest)
        let mut ver_msg: Vec<u8> = Vec::new();
        write_varint_field(&mut ver_msg, 1, version_code);
        write_varint_field(&mut ver_msg, 3, 0);

        // Build Message30 (field 30 of AcquireRequest)
        let mut m30_msg: Vec<u8> = Vec::new();
        write_varint_field(&mut m30_msg, 1, 2);
        write_varint_field(&mut m30_msg, 2, 0);

        // Generate nonce
        use base64::Engine;
        let mut rand_bytes = [0u8; 256];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut rand_bytes);
        let nonce = format!(
            "nonce={}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&rand_bytes)
        );

        // Assemble root AcquireRequest
        let mut root: Vec<u8> = Vec::new();
        write_msg(&mut root, 1, pkg_msg);
        write_msg(&mut root, 12, ver_msg);
        write_varint_field(&mut root, 13, offer_type as u64);
        write_varint_field(&mut root, 15, 0);
        write_string(&mut root, 22, &nonce);
        write_varint_field(&mut root, 25, 2);
        write_msg(&mut root, 30, m30_msg);

        root
    }
}

pub struct ExodusApi {
    client: Client,
    api_key: String,
}

impl ExodusApi {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let api_key =
            std::env::var("EXODUS_API_KEY").unwrap_or_else(|_| DEFAULT_EXODUS_API_KEY.to_string());
        Self { client, api_key }
    }

    /// Search for historical version reports by package name
    pub async fn fetch_versions(&self, package_name: &str) -> Result<Vec<VersionReport>> {
        let url = format!("{}/{}", EXODUS_SEARCH_URL, package_name);

        let res = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Token {}", self.api_key))
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Exodus API request failed: HTTP {}",
                res.status()
            ));
        }

        let val: Value = res.json().await?;
        let reports_array = val
            .get(package_name)
            .and_then(|obj| obj.get("reports"))
            .and_then(|r| r.as_array());

        let mut reports = Vec::new();
        if let Some(arr) = reports_array {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                let version_name = item
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let version_code_str = item
                    .get("version_code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let version_code = version_code_str.parse::<u64>().unwrap_or(0);
                let release_date = item
                    .get("creation_date")
                    .or_else(|| item.get("updated_at"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let trackers_count = item
                    .get("trackers")
                    .and_then(|v| v.as_array())
                    .map(|t| t.len())
                    .unwrap_or(0);

                reports.push(VersionReport {
                    id,
                    version_name,
                    version_code,
                    release_date,
                    trackers_count,
                    source: "Exodus Privacy".to_string(),
                });
            }
        }

        reports.sort_by(|a, b| b.version_code.cmp(&a.version_code));
        Ok(reports)
    }
}
