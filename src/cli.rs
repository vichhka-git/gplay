use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gplay-cli",
    author = "vichhka-git",
    version = "0.1.0",
    about = "Fast, standalone Google Play Store APK downloader & installer in Rust",
    long_about = "gplay-cli allows searching, listing versions, and downloading authentic, original Google Play Store APKs without needing a Google sign-in. Supports standalone APKs, split APK bundles (.apks), and direct one-click ADB installation."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Verbose output mode
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for applications on Google Play Store
    Search(SearchArgs),

    /// Display details of an app
    Info(InfoArgs),

    /// List available historical versions and version codes
    Versions(VersionsArgs),

    /// Download APK for an app (e.g. gplay download <pkg> [version])
    Download(DownloadArgs),

    /// Download and/or install an app or APK file directly via ADB
    Install(InstallArgs),

    /// Manage authentication sessions
    Auth(AuthArgs),

    /// Launch the interactive search and download wizard
    Interactive,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query (app name or keywords)
    pub query: String,

    /// Maximum number of search results to display
    #[arg(short, long, default_value_t = 15)]
    pub limit: usize,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Package name (e.g. org.thoughtcrime.securesms)
    pub package_name: String,
}

#[derive(Args, Debug)]
pub struct VersionsArgs {
    /// Package name (e.g. org.thoughtcrime.securesms)
    pub package_name: String,
}

#[derive(Args, Debug)]
pub struct DownloadArgs {
    /// Package name (e.g. org.thoughtcrime.securesms)
    pub package_name: String,

    /// Target version name or code (e.g. 8.22.2, 173301, or latest)
    pub version: Option<String>,

    /// Target version name or code via flag (e.g. --version 8.22.2)
    #[arg(long = "version")]
    pub version_flag: Option<String>,

    /// Output directory (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,

    /// Do not create a .apks bundle archive for split APKs
    #[arg(long)]
    pub no_bundle: bool,
}

impl DownloadArgs {
    pub fn requested_version(&self) -> Option<&str> {
        self.version_flag.as_deref().or(self.version.as_deref())
    }
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Target package name (to download & install) OR local path to .apk / .apks / folder of splits
    pub target: String,

    /// Target version name or code (if target is a package name)
    pub version: Option<String>,

    /// Target version name or code via flag
    #[arg(long = "version")]
    pub version_flag: Option<String>,

    /// Output directory for downloaded files (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,

    /// Specific ADB device serial (optional)
    #[arg(short = 's', long)]
    pub device: Option<String>,
}

impl InstallArgs {
    pub fn requested_version(&self) -> Option<&str> {
        self.version_flag.as_deref().or(self.version.as_deref())
    }
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Show current cached session status
    Status,

    /// Log in with your custom Google Account (interactive browser / token login)
    Login(LoginArgs),

    /// Force refresh and obtain a new anonymous session token (for anonymous sessions)
    Refresh,

    /// Clear cached session
    Logout,
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Google email address (optional, prompted interactively if omitted)
    #[arg(short, long)]
    pub email: Option<String>,

    /// Google auth token or OAuth token (optional, prompted interactively or opened in browser)
    #[arg(short, long)]
    pub token: Option<String>,
}
