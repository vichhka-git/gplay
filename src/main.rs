mod adb;
mod api;
mod apks;
mod auth;
mod cli;
mod device;
mod downloader;
mod interactive;
mod models;
mod proto;

use anyhow::Result;
use clap::Parser;
use cli::{
    AuthAction, Cli, Commands, DownloadArgs, InfoArgs, InstallArgs, SearchArgs, VersionsArgs,
};
use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, ContentArrangement, Table};
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Search(args)) => handle_search(args).await,
        Some(Commands::Info(args)) => handle_info(args).await,
        Some(Commands::Versions(args)) => handle_versions(args).await,
        Some(Commands::Download(args)) => handle_download(args).await,
        Some(Commands::Install(args)) => handle_install(args).await,
        Some(Commands::Auth(args)) => handle_auth(args).await,
        Some(Commands::Interactive) | None => interactive::run_interactive().await,
    }
}

async fn handle_search(args: SearchArgs) -> Result<()> {
    println!(
        "{}",
        format!("🔍 Searching Google Play for '{}'...", args.query).cyan()
    );

    let auth_mgr = auth::AuthManager::default();
    let play_client = api::GooglePlayApi::new(auth_mgr);

    let mut results = play_client.search(&args.query).await?;

    if results.is_empty() {
        println!("{}", "No applications found.".yellow());
        return Ok(());
    }

    // Enrich top results concurrently with real download counts and metadata
    let to_enrich = std::cmp::min(args.limit, results.len());
    let mut tasks = Vec::new();
    for i in 0..to_enrich {
        let pkg = results[i].package_name.clone();
        let client = &play_client;
        tasks.push(async move { client.details(&pkg).await.ok() });
    }

    let enriched = futures_util::future::join_all(tasks).await;
    for (i, opt) in enriched.into_iter().enumerate() {
        if let Some(details) = opt {
            if details.downloads_text != "N/A" {
                results[i].downloads_text = details.downloads_text;
            }
            if details.rating > 0.0 {
                results[i].rating = details.rating;
            }
            if !details.category.is_empty() && details.category != "Application" {
                results[i].category = details.category;
            }
        }
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#").fg(Color::DarkGrey),
            Cell::new("App Title").fg(Color::Cyan),
            Cell::new("Package Name").fg(Color::Green),
            Cell::new("Developer").fg(Color::Yellow),
            Cell::new("Rating").fg(Color::Magenta),
            Cell::new("Downloads").fg(Color::Blue),
        ]);

    for (idx, app) in results.iter().take(args.limit).enumerate() {
        let stars = if app.rating > 0.0 {
            format!("{:.1} ★", app.rating)
        } else {
            "N/A".to_string()
        };

        table.add_row(vec![
            Cell::new((idx + 1).to_string()),
            Cell::new(&app.title),
            Cell::new(&app.package_name),
            Cell::new(&app.developer),
            Cell::new(stars),
            Cell::new(&app.downloads_text),
        ]);
    }

    println!("{table}");
    Ok(())
}

async fn handle_info(args: InfoArgs) -> Result<()> {
    println!(
        "{}",
        format!("📦 Fetching details for '{}'...", args.package_name).cyan()
    );

    let auth_mgr = auth::AuthManager::default();
    let play_client = api::GooglePlayApi::new(auth_mgr);

    let details = play_client.details(&args.package_name).await?;

    println!("\n{}", format!("📱 {}", details.title).green().bold());
    println!("{}", "═".repeat(details.title.len() + 3).green());
    println!("{}: {}", "Package Name".bold(), details.package_name);
    println!("{}: {}", "Developer".bold(), details.developer);
    println!(
        "{}: {} (versionCode: {})",
        "Latest Version".bold(),
        details.version_name.cyan(),
        details.version_code.to_string().yellow()
    );
    println!("{}: {}", "Downloads".bold(), details.downloads_text);
    println!("{}: {:.1} ★", "Rating".bold(), details.rating);
    if !details.category.is_empty() {
        println!("{}: {}", "Category".bold(), details.category);
    }
    if let Some(ref desc) = details.description {
        println!("\n{}:\n{}", "Description".bold(), desc);
    }

    Ok(())
}

async fn handle_versions(args: VersionsArgs) -> Result<()> {
    println!(
        "{}",
        format!("📜 Fetching version history for '{}'...", args.package_name).cyan()
    );

    let exodus_api = api::ExodusApi::new();
    let versions = exodus_api.fetch_versions(&args.package_name).await?;

    if versions.is_empty() {
        println!(
            "{}",
            "No historical versions archived in public registry. You can download the latest version using 'gplay-cli download <pkg>'."
                .yellow()
        );
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Version Name").fg(Color::Cyan),
            Cell::new("Version Code").fg(Color::Green),
            Cell::new("Release Date").fg(Color::Yellow),
            Cell::new("Trackers").fg(Color::Magenta),
        ]);

    for v in &versions {
        table.add_row(vec![
            Cell::new(&v.version_name),
            Cell::new(v.version_code.to_string()),
            Cell::new(&v.release_date),
            Cell::new(v.trackers_count.to_string()),
        ]);
    }

    println!("{table}");
    println!(
        "\n{}",
        format!(
            "💡 Tip: Run 'gplay-cli download {} <VERSION>' (e.g. gplay-cli download {} {}) to download an exact release.",
            args.package_name,
            args.package_name,
            versions.first().map(|v| v.version_name.as_str()).unwrap_or("8.22.2")
        )
        .italic()
    );

    Ok(())
}

async fn handle_download(args: DownloadArgs) -> Result<()> {
    let auth_mgr = auth::AuthManager::default();
    let play_client = api::GooglePlayApi::new(auth_mgr);

    let resolved = play_client
        .resolve_version(&args.package_name, args.requested_version())
        .await?;

    println!(
        "{}",
        format!(
            "🚀 Requesting delivery for {} (version: {}, code: {})...",
            args.package_name.bold(),
            resolved.version_name.green().bold(),
            resolved.version_code.to_string().yellow()
        )
        .cyan()
    );

    let delivery = play_client
        .acquire_and_deliver(&args.package_name, resolved.version_code, 1)
        .await?;

    let downloader = downloader::Downloader::new();
    let is_split = !delivery.split_files.is_empty();

    // Clean version string for filesystem
    let ver_tag = resolved.version_name.replace(['/', '\\', ' '], "_");

    if is_split {
        println!(
            "Files to download: Base APK + {} split APKs ({:.2} MB total)",
            delivery.split_files.len(),
            delivery.total_size as f64 / (1024.0 * 1024.0)
        );

        let app_folder = args
            .output_dir
            .join(format!("{}_{}", args.package_name, ver_tag));
        let downloaded_paths = downloader.download_delivery(&delivery, &app_folder).await?;

        println!(
            "{}",
            "✅ All split files downloaded successfully!".green().bold()
        );
        println!(
            "{}",
            format!("📂 Splits saved to: {}", app_folder.display()).green()
        );

        let bundle_path = args
            .output_dir
            .join(format!("{}_{}.apks", args.package_name, ver_tag));
        if !args.no_bundle {
            apks::ApksBundler::create_bundle(&downloaded_paths, &bundle_path)?;
            println!(
                "{}",
                format!(
                    "📦 Standard .apks bundle created: {}",
                    bundle_path.display()
                )
                .cyan()
                .bold()
            );
        }
    } else {
        let temp_dir = tempfile::tempdir()?;
        let downloaded_paths = downloader
            .download_delivery(&delivery, temp_dir.path())
            .await?;

        println!("{}", "✅ Download completed!".green().bold());

        if let Some(base_file) = downloaded_paths.first() {
            std::fs::create_dir_all(&args.output_dir)?;

            // Check if downloaded archive is an XAPK/APKS multi-split bundle
            let app_folder = args
                .output_dir
                .join(format!("{}_{}", args.package_name, ver_tag));
            if let Ok(Some(splits)) = inspect_and_unpack_bundle(base_file, &app_folder) {
                println!(
                    "{}",
                    format!("📦 Detected multi-split bundle ({} APKs).", splits.len())
                        .cyan()
                        .bold()
                );
                println!(
                    "{}",
                    format!("📂 Splits saved to: {}", app_folder.display()).green()
                );

                let bundle_path = args
                    .output_dir
                    .join(format!("{}_{}.apks", args.package_name, ver_tag));
                if !args.no_bundle {
                    apks::ApksBundler::create_bundle(&splits, &bundle_path)?;
                    println!(
                        "{}",
                        format!(
                            "📦 Standard .apks bundle created: {}",
                            bundle_path.display()
                        )
                        .cyan()
                        .bold()
                    );
                }
            } else {
                let final_apk = args
                    .output_dir
                    .join(format!("{}_{}.apk", args.package_name, ver_tag));
                std::fs::copy(base_file, &final_apk)?;
                println!(
                    "{}",
                    format!("🎉 APK saved to: {}", final_apk.display())
                        .green()
                        .bold()
                );
            }
        }
    }

    Ok(())
}

async fn handle_install(args: InstallArgs) -> Result<()> {
    let target_path = Path::new(&args.target);

    // 1. Local .apk / .xapk file
    if target_path.is_file() && (args.target.ends_with(".apk") || args.target.ends_with(".xapk")) {
        println!(
            "{}",
            format!("📦 Installing local package: {}", target_path.display()).cyan()
        );
        return adb::Adb::install_auto(target_path, args.device.as_deref());
    }

    // 2. Local .apks bundle
    if target_path.is_file() && args.target.ends_with(".apks") {
        println!(
            "{}",
            format!(
                "📦 Installing local .apks bundle: {}",
                target_path.display()
            )
            .cyan()
        );
        return adb::Adb::install_apks(target_path, args.device.as_deref());
    }

    // 3. Local directory containing split APKs
    if target_path.is_dir() {
        println!(
            "{}",
            format!(
                "📂 Installing split APKs from directory: {}",
                target_path.display()
            )
            .cyan()
        );
        let mut split_paths = Vec::new();
        for entry in std::fs::read_dir(target_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("apk") {
                split_paths.push(path);
            }
        }

        if split_paths.is_empty() {
            return Err(anyhow::anyhow!(
                "No .apk files found in directory {:?}",
                target_path
            ));
        }

        return adb::Adb::install_splits(&split_paths, args.device.as_deref());
    }

    // 4. Target is a package name: download and install directly!
    println!(
        "{}",
        format!(
            "⬇️  Downloading and installing '{}' from Google Play...",
            args.target
        )
        .cyan()
    );

    let auth_mgr = auth::AuthManager::default();
    let play_client = api::GooglePlayApi::new(auth_mgr);

    let resolved = play_client
        .resolve_version(&args.target, args.requested_version())
        .await?;

    println!(
        "{}",
        format!(
            "🚀 Requesting delivery for {} (version: {}, code: {})...",
            args.target.bold(),
            resolved.version_name.green().bold(),
            resolved.version_code.to_string().yellow()
        )
        .cyan()
    );

    let delivery = play_client
        .acquire_and_deliver(&args.target, resolved.version_code, 1)
        .await?;

    let downloader = downloader::Downloader::new();
    let is_split = !delivery.split_files.is_empty();
    let ver_tag = resolved.version_name.replace(['/', '\\', ' '], "_");

    if is_split {
        println!(
            "Files to download: Base APK + {} split APKs ({:.2} MB total)",
            delivery.split_files.len(),
            delivery.total_size as f64 / (1024.0 * 1024.0)
        );

        let app_folder = args.output_dir.join(format!("{}_{}", args.target, ver_tag));
        let downloaded_paths = downloader.download_delivery(&delivery, &app_folder).await?;

        println!(
            "{}",
            "✅ All split files downloaded successfully!".green().bold()
        );
        println!(
            "{}",
            format!("📂 Splits saved to: {}", app_folder.display()).green()
        );

        let bundle_path = args
            .output_dir
            .join(format!("{}_{}.apks", args.target, ver_tag));
        apks::ApksBundler::create_bundle(&downloaded_paths, &bundle_path)?;
        println!(
            "{}",
            format!(
                "📦 Standard .apks bundle created: {}",
                bundle_path.display()
            )
            .cyan()
            .bold()
        );

        println!(
            "{}",
            "📲 Installing via ADB (adb install-multiple)...".cyan()
        );
        adb::Adb::install_splits(&downloaded_paths, args.device.as_deref())?;
    } else {
        let size_mb = delivery.total_size as f64 / (1024.0 * 1024.0);
        if size_mb > 0.0 {
            println!("Files to download: Package ({:.2} MB)", size_mb);
        } else {
            println!("Files to download: Package");
        }

        let temp_dir = tempfile::tempdir()?;
        let downloaded_paths = downloader
            .download_delivery(&delivery, temp_dir.path())
            .await?;

        println!("{}", "✅ Download completed!".green().bold());

        if let Some(base_file) = downloaded_paths.first() {
            std::fs::create_dir_all(&args.output_dir)?;

            // Check if downloaded archive is an XAPK/APKS multi-split bundle
            let app_folder = args.output_dir.join(format!("{}_{}", args.target, ver_tag));
            if let Ok(Some(splits)) = inspect_and_unpack_bundle(base_file, &app_folder) {
                println!(
                    "{}",
                    format!("📦 Detected multi-split bundle ({} APKs).", splits.len())
                        .cyan()
                        .bold()
                );
                println!(
                    "{}",
                    format!("📂 Splits saved to: {}", app_folder.display()).green()
                );

                let bundle_path = args
                    .output_dir
                    .join(format!("{}_{}.apks", args.target, ver_tag));
                apks::ApksBundler::create_bundle(&splits, &bundle_path)?;
                println!(
                    "{}",
                    format!(
                        "📦 Standard .apks bundle created: {}",
                        bundle_path.display()
                    )
                    .cyan()
                    .bold()
                );

                println!(
                    "{}",
                    "📲 Installing multi-split bundle via ADB (adb install-multiple)...".cyan()
                );
                adb::Adb::install_splits(&splits, args.device.as_deref())?;
            } else {
                let final_apk = args
                    .output_dir
                    .join(format!("{}_{}.apk", args.target, ver_tag));
                std::fs::copy(base_file, &final_apk)?;
                println!(
                    "{}",
                    format!("🎉 APK saved to: {}", final_apk.display())
                        .green()
                        .bold()
                );

                println!("{}", "📲 Installing standalone APK via ADB...".cyan());
                adb::Adb::install_single(&final_apk, args.device.as_deref())?;
            }
        }
    }

    Ok(())
}

fn inspect_and_unpack_bundle(
    file_path: &Path,
    output_folder: &Path,
) -> Result<Option<Vec<PathBuf>>> {
    let file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return Ok(None),
    };

    let mut apk_names = Vec::new();
    let mut obb_names = Vec::new();
    let mut has_manifest = false;

    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            if name.ends_with(".apk") {
                apk_names.push(name);
            } else if name.ends_with(".obb") {
                obb_names.push(name);
            } else if name == "manifest.json" {
                has_manifest = true;
            }
        }
    }

    if apk_names.len() > 1 || (has_manifest && !apk_names.is_empty()) || !obb_names.is_empty() {
        std::fs::create_dir_all(output_folder)?;
        let mut extracted = Vec::new();
        for name in &apk_names {
            let mut entry = archive.by_name(name)?;
            let file_name = Path::new(name).file_name().unwrap_or_default();
            let out_path = output_folder.join(file_name);
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
            extracted.push(out_path);
        }

        for name in &obb_names {
            if let Ok(mut entry) = archive.by_name(name) {
                let out_path = output_folder.join(name);
                if let Some(parent) = out_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(mut out_file) = std::fs::File::create(&out_path) {
                    let _ = std::io::copy(&mut entry, &mut out_file);
                }
            }
        }

        return Ok(Some(extracted));
    }

    Ok(None)
}

async fn handle_auth(args: cli::AuthArgs) -> Result<()> {
    let auth_mgr = auth::AuthManager::default();

    match args.action {
        AuthAction::Status => {
            if let Some(session) = auth::AuthManager::load_cached_session() {
                println!("{}", "🔑 Current Authentication Session:".green().bold());
                println!(
                    "Type: {}",
                    if session.is_custom {
                        "Custom Google Account (User Login)".green().bold()
                    } else {
                        "Anonymous (Aurora Token Dispenser)".cyan().bold()
                    }
                );
                println!("Email: {}", session.email.cyan());
                println!("GSF ID: {}", session.gsf_id.yellow());
                println!("Token Length: {} bytes", session.auth_token.len());
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let elapsed = now.saturating_sub(session.created_at);
                println!("Session Age: {} minutes", elapsed / 60);
            } else {
                println!("{}", "No cached session found. Run 'gplay-cli auth login' to sign in, or 'gplay-cli auth refresh' for anonymous mode.".yellow());
            }
        }
        AuthAction::Login(login_args) => {
            println!("{}", "🔐 Google Account Login".green().bold());
            println!("{}", "───────────────────────".green());

            let (email, token) = match (login_args.email, login_args.token) {
                (Some(e), Some(t)) => (e, t),
                (opt_e, opt_t) => {
                    let mut e = opt_e.unwrap_or_default();
                    let mut t = opt_t.unwrap_or_default();

                    if e.is_empty() {
                        e = inquire::Text::new("Google Email Address:")
                            .with_placeholder("e.g. user@gmail.com")
                            .prompt()?;
                    }

                    if t.is_empty() {
                        println!("\n{}", "🌐 Browser Sign-In:".cyan().bold());
                        println!(
                            "1. Open this official Google Account login URL in your browser:\n"
                        );
                        let login_url = "https://accounts.google.com/EmbeddedSetup";
                        println!("   👉 {}\n", login_url.yellow().bold());

                        // Attempt to open in system browser automatically
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open").arg(login_url).spawn();
                        #[cfg(target_os = "linux")]
                        let _ = std::process::Command::new("xdg-open")
                            .arg(login_url)
                            .spawn();
                        #[cfg(target_os = "windows")]
                        let _ = std::process::Command::new("cmd")
                            .args(&["/c", "start", login_url])
                            .spawn();

                        println!("2. Sign in to your Google Account.");
                        println!("3. Copy the 'oauth_token' cookie or authentication token from the browser page/URL.");
                        println!("   (Or use an OAuth / AAS token starting with 'oauth2_4/' or similar).\n");

                        t = inquire::Password::new("Paste Google Auth / OAuth Token:")
                            .with_display_mode(inquire::PasswordDisplayMode::Masked)
                            .without_confirmation()
                            .prompt()?;
                    }
                    (e, t)
                }
            };

            println!(
                "\n{}",
                "⏳ Verifying credentials and registering device configuration with Google Play..."
                    .cyan()
            );
            match auth_mgr.login_custom_session(&email, &token).await {
                Ok(session) => {
                    println!(
                        "\n{}",
                        format!("✅ Successfully logged in as '{}'!", session.email)
                            .green()
                            .bold()
                    );
                    println!("📱 Device GSF ID: {}", session.gsf_id.yellow());
                    println!(
                        "🔒 Custom session stored securely in ~/.config/gplay-cli/session.json"
                    );
                    println!("💡 All future searches, downloads, and installs will use this Google account.\n");
                }
                Err(e) => {
                    println!(
                        "\n{}",
                        format!("❌ Failed to log in with provided credentials: {}", e)
                            .red()
                            .bold()
                    );
                    return Err(e);
                }
            }
        }
        AuthAction::Refresh => {
            if let Some(session) = auth::AuthManager::load_cached_session() {
                if session.is_custom {
                    println!(
                        "{}",
                        format!(
                            "⚠️  Active session is a custom Google Account ({})",
                            session.email
                        )
                        .yellow()
                        .bold()
                    );
                    println!(
                        "To re-authenticate or update your token, run 'gplay-cli auth login'."
                    );
                    return Ok(());
                }
            }
            println!(
                "{}",
                "🔄 Requesting new token from Aurora token dispenser...".cyan()
            );
            let session = auth_mgr.get_session(true).await?;
            println!(
                "{}",
                "✅ Anonymous session successfully refreshed!"
                    .green()
                    .bold()
            );
            println!("Email: {}", session.email.cyan());
            println!("GSF ID: {}", session.gsf_id.yellow());
        }
        AuthAction::Logout => {
            auth::AuthManager::clear_session()?;
            println!("{}", "✅ Session cleared successfully.".green());
        }
    }

    Ok(())
}
