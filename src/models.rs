use serde::{Deserialize, Serialize};

/// Basic information about an application from Google Play or search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub package_name: String,
    pub title: String,
    pub developer: String,
    pub version_name: String,
    pub version_code: u64,
    pub size_bytes: u64,
    pub category: String,
    pub rating: f32,
    pub downloads_text: String,
    pub icon_url: Option<String>,
    pub description: Option<String>,
}

/// Historical version record retrieved from Exodus Privacy API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionReport {
    pub id: u64,
    pub version_name: String,
    pub version_code: u64,
    pub release_date: String,
    pub trackers_count: usize,
    pub source: String,
}

/// Spoofed Android device profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub name: String,
    pub fingerprint: String,
    pub hardware: String,
    pub radio: String,
    pub brand: String,
    pub device: String,
    pub sdk_int: u32,
    pub release: String,
    pub model: String,
    pub manufacturer: String,
    pub product: String,
    pub id: String,
    pub bootloader: String,
    pub density: u32,
    pub width: u32,
    pub height: u32,
    pub platforms: Vec<String>,
    pub features: Vec<String>,
    pub locales: Vec<String>,
    pub shared_libraries: Vec<String>,
    pub gl_version: u32,
    pub gl_extensions: Vec<String>,
    pub gsf_version: u64,
    pub vending_version: u64,
    pub vending_version_string: String,
    pub client: String,
    pub roaming: String,
    pub timezone: String,
    pub cell_operator: String,
    pub sim_operator: String,
}

/// Cached authentication session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub email: String,
    pub auth_token: String,
    pub gsf_id: String,
    pub user_agent: String,
    #[serde(default)]
    pub device_config_token: Option<String>,
    #[serde(default)]
    pub device_checkin_consistency_token: Option<String>,
    #[serde(default)]
    pub dfe_cookie: Option<String>,
    pub dispenser_url: String,
    pub created_at: u64,
    #[serde(default)]
    pub is_custom: bool,
}

/// Downloadable file entry from Google Play delivery response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayDownloadFile {
    pub name: String,
    pub file_type: DownloadFileType,
    pub size_bytes: u64,
    pub download_url: String,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadFileType {
    BaseApk,
    SplitConfig,
    DynamicFeature,
    ObbMain,
    ObbPatch,
}

/// Resulting delivery payload for an app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDelivery {
    pub package_name: String,
    pub version_code: u64,
    pub total_size: u64,
    pub base_file: PlayDownloadFile,
    pub split_files: Vec<PlayDownloadFile>,
}
