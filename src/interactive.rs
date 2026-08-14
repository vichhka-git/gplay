use crate::adb::Adb;
use crate::api::{ExodusApi, GooglePlayApi};
use crate::apks::ApksBundler;
use crate::auth::AuthManager;
use crate::downloader::Downloader;
use anyhow::Result;
use colored::*;
use inquire::{Select, Text};
use std::path::PathBuf;

pub async fn run_interactive() -> Result<()> {
    println!(
        "{}",
        "=================================================="
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "  🚀 gplay-cli - Google Play APK Downloader & Installer "
            .green()
            .bold()
    );
    println!(
        "{}",
        "=================================================="
            .cyan()
            .bold()
    );
    println!();

    let auth_mgr = AuthManager::default();
    let play_client = GooglePlayApi::new(auth_mgr);

    // 1. Search Query
    let query = Text::new("Search app on Google Play:")
        .with_placeholder("e.g. Telegram, Signal, WhatsApp, Spotify")
        .prompt()?;

    if query.trim().is_empty() {
        println!("{}", "Search query cannot be empty.".yellow());
        return Ok(());
    }

    println!("{}", format!("🔍 Searching for '{}'...", query).blue());
    let mut apps = play_client.search(&query).await?;

    if apps.is_empty() {
        println!("{}", "No matching apps found.".red());
        return Ok(());
    }

    // Enrich top results concurrently
    let to_enrich = std::cmp::min(15, apps.len());
    let mut tasks = Vec::new();
    for i in 0..to_enrich {
        let pkg = apps[i].package_name.clone();
        let client = &play_client;
        tasks.push(async move { client.details(&pkg).await.ok() });
    }

    let enriched = futures_util::future::join_all(tasks).await;
    for (i, opt) in enriched.into_iter().enumerate() {
        if let Some(details) = opt {
            if details.downloads_text != "N/A" {
                apps[i].downloads_text = details.downloads_text;
            }
            if details.rating > 0.0 {
                apps[i].rating = details.rating;
            }
        }
    }

    // 2. Select App
    let app_options: Vec<String> = apps
        .iter()
        .map(|a| {
            let stars = if a.rating > 0.0 {
                format!("{:.1} ★", a.rating)
            } else {
                "".to_string()
            };
            let dl = if a.downloads_text != "N/A" {
                format!(" ({})", a.downloads_text)
            } else {
                "".to_string()
            };
            format!(
                "{} ({}) - {}{}{}",
                a.title,
                a.package_name,
                a.developer,
                if stars.is_empty() { "" } else { " " },
                stars
            ) + &dl
        })
        .collect();

    let selected_idx = Select::new("Select an app:", app_options).raw_prompt()?;
    let selected_app = &apps[selected_idx.index];

    println!();
    println!(
        "{} {}",
        "Selected:".bold(),
        selected_app.title.green().bold()
    );
    println!("{} {}", "Package:".bold(), selected_app.package_name.cyan());
    println!();

    // 3. Fetch Versions
    println!(
        "{}",
        "📦 Fetching historical versions from public registry...".blue()
    );
    let exodus_api = ExodusApi::new();
    let versions_result = exodus_api.fetch_versions(&selected_app.package_name).await;

    let mut version_options = Vec::new();
    version_options.push("⭐ Latest Version (from Google Play)".to_string());

    let reports = versions_result.unwrap_or_default();
    for r in &reports {
        version_options.push(format!(
            "v{} (code: {}) - Released: {} (Trackers: {})",
            r.version_name, r.version_code, r.release_date, r.trackers_count
        ));
    }
    version_options.push("✏️  Enter custom version name / code manually...".to_string());

    let ver_select = Select::new("Choose version to download:", version_options).raw_prompt()?;

    let (target_version_code, version_label) = if ver_select.index == 0 {
        // Latest from details
        let details = play_client.details(&selected_app.package_name).await?;
        println!(
            "Latest version: {} (code: {})",
            details.version_name.green(),
            details.version_code.to_string().yellow().bold()
        );
        (details.version_code, details.version_name)
    } else if ver_select.index <= reports.len() {
        let rep = &reports[ver_select.index - 1];
        (rep.version_code, rep.version_name.clone())
    } else {
        // Manual entry: resolve version name or code
        let manual_input =
            Text::new("Enter version name (e.g. 8.22.2) or integer versionCode:").prompt()?;
        let resolved = play_client
            .resolve_version(&selected_app.package_name, Some(&manual_input))
            .await?;
        (resolved.version_code, resolved.version_name)
    };

    // 4. Choose Action (Download vs Download & Install)
    let action_options = vec![
        "💾 Download APK / Split files + Standard .apks bundle".to_string(),
        "📲 Download and Install directly on connected ADB device".to_string(),
    ];
    let action_select = Select::new("Action:", action_options).raw_prompt()?;
    let direct_install = action_select.index == 1;

    // 5. Output directory (default to current directory)
    let default_output = ".";
    let output_dir_str = Text::new("Output directory:")
        .with_default(default_output)
        .prompt()?;
    let output_dir = PathBuf::from(output_dir_str);

    // 6. Execute Download
    println!();
    println!(
        "{}",
        format!(
            "🚀 Requesting delivery for {} (version: {}, code: {})...",
            selected_app.package_name,
            version_label.green(),
            target_version_code
        )
        .green()
        .bold()
    );

    let delivery = play_client
        .acquire_and_deliver(&selected_app.package_name, target_version_code, 1)
        .await?;

    let is_split = !delivery.split_files.is_empty();
    let downloader = Downloader::new();
    let ver_tag = version_label.replace(['/', '\\', ' '], "_");

    if is_split {
        println!(
            "Files to download: Base APK + {} split APKs ({:.2} MB total)",
            delivery.split_files.len(),
            delivery.total_size as f64 / (1024.0 * 1024.0)
        );

        let app_folder = output_dir.join(format!("{}_{}", selected_app.package_name, ver_tag));
        let downloaded_paths = downloader.download_delivery(&delivery, &app_folder).await?;

        println!("{}", "✅ Download completed successfully!".green().bold());
        println!(
            "{}",
            format!("📂 Split APK files saved to: {}", app_folder.display())
                .green()
                .bold()
        );

        let bundle_path =
            output_dir.join(format!("{}_{}.apks", selected_app.package_name, ver_tag));
        ApksBundler::create_bundle(&downloaded_paths, &bundle_path)?;
        println!(
            "{}",
            format!(
                "📦 Standard .apks bundle created: {}",
                bundle_path.display()
            )
            .cyan()
            .bold()
        );

        if direct_install {
            println!(
                "{}",
                "📲 Installing via ADB (adb install-multiple)...".cyan()
            );
            Adb::install_splits(&downloaded_paths, None)?;
        }
    } else {
        println!(
            "Files to download: Standalone APK ({:.2} MB)",
            delivery.total_size as f64 / (1024.0 * 1024.0)
        );

        let temp_dir = tempfile::tempdir()?;
        let downloaded_paths = downloader
            .download_delivery(&delivery, temp_dir.path())
            .await?;

        println!("{}", "✅ Download completed successfully!".green().bold());

        if let Some(base_file) = downloaded_paths.first() {
            std::fs::create_dir_all(&output_dir)?;
            let final_apk =
                output_dir.join(format!("{}_{}.apk", selected_app.package_name, ver_tag));
            std::fs::copy(base_file, &final_apk)?;
            println!(
                "{}",
                format!("🎉 Standalone APK saved: {}", final_apk.display())
                    .green()
                    .bold()
            );

            if direct_install {
                println!("{}", "📲 Installing standalone APK via ADB...".cyan());
                Adb::install_single(&final_apk, None)?;
            }
        }
    }

    println!();
    Ok(())
}
