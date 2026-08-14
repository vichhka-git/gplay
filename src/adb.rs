use anyhow::{Context, Result};
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

pub struct Adb;

impl Adb {
    /// Detects connected ADB devices
    pub fn get_connected_devices() -> Result<Vec<String>> {
        let output = Command::new("adb")
            .arg("devices")
            .output()
            .context("Failed to execute 'adb devices'. Is ADB installed and in your PATH?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ADB command failed: {}", stderr.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                devices.push(parts[0].to_string());
            }
        }

        Ok(devices)
    }

    /// Selects a target device (specified, single detected, or waits for device connection)
    pub fn resolve_device(specified: Option<&str>) -> Result<String> {
        let mut devices = Self::get_connected_devices()?;

        if devices.is_empty() {
            println!(
                "{}",
                "⏳ No connected ADB device detected. Waiting for device to connect via USB/Wi-Fi... (Press Ctrl+C to cancel)"
                    .yellow()
            );

            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(120);

            while devices.is_empty() {
                if start.elapsed() > timeout {
                    return Err(anyhow::anyhow!(
                        "Timed out waiting for ADB device (120s). Connect a device via USB/Wi-Fi and enable USB debugging."
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(1000));
                if let Ok(devs) = Self::get_connected_devices() {
                    devices = devs;
                }
            }

            println!(
                "{}",
                format!("📱 Device detected: '{}'", devices[0])
                    .green()
                    .bold()
            );
        }

        if let Some(target) = specified {
            if devices.iter().any(|d| d == target) {
                return Ok(target.to_string());
            } else {
                return Err(anyhow::anyhow!(
                    "Device '{}' not found. Available devices: {}",
                    target,
                    devices.join(", ")
                ));
            }
        }

        Ok(devices[0].clone())
    }

    /// Automatically detects if a file is a standalone APK or a bundle/zip containing multiple split APKs (XAPK/APKS),
    /// extracts components and OBB expansion files, and installs accordingly.
    pub fn install_auto(file_path: &Path, device_id: Option<&str>) -> Result<()> {
        if let Ok(file) = std::fs::File::open(file_path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                let mut apk_entry_names = Vec::new();
                let mut obb_entry_names = Vec::new();
                let mut has_manifest = false;

                for i in 0..archive.len() {
                    if let Ok(entry) = archive.by_index(i) {
                        let name = entry.name().to_string();
                        if name.ends_with(".apk") {
                            apk_entry_names.push(name);
                        } else if name.ends_with(".obb") {
                            obb_entry_names.push(name);
                        } else if name == "manifest.json" {
                            has_manifest = true;
                        }
                    }
                }

                // If it's a multi-split bundle or an XAPK containing nested APKs/OBBs
                if apk_entry_names.len() > 1
                    || (has_manifest && !apk_entry_names.is_empty())
                    || !obb_entry_names.is_empty()
                {
                    let temp_dir = tempfile::tempdir()?;
                    let mut extracted_apks = Vec::new();

                    for name in &apk_entry_names {
                        let mut entry = archive.by_name(name)?;
                        let out_name = Path::new(name).file_name().unwrap_or_default();
                        let out_path = temp_dir.path().join(out_name);
                        let mut out_file = std::fs::File::create(&out_path)?;
                        std::io::copy(&mut entry, &mut out_file)?;
                        extracted_apks.push(out_path);
                    }

                    // Push OBB files to device if any
                    let target_device = Self::resolve_device(device_id)?;
                    for name in &obb_entry_names {
                        if let Ok(mut entry) = archive.by_name(name) {
                            let out_path = temp_dir.path().join(name);
                            if let Some(parent) = out_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            if let Ok(mut out_file) = std::fs::File::create(&out_path) {
                                if std::io::copy(&mut entry, &mut out_file).is_ok() {
                                    println!(
                                        "{}",
                                        format!(
                                            "📂 Pushing expansion file '{}' to device...",
                                            name
                                        )
                                        .cyan()
                                    );
                                    let remote_dest = format!("/sdcard/{}", name);
                                    let _ = Command::new("adb")
                                        .args([
                                            "-s",
                                            &target_device,
                                            "push",
                                            out_path.to_str().unwrap_or_default(),
                                            &remote_dest,
                                        ])
                                        .status();
                                }
                            }
                        }
                    }

                    if extracted_apks.len() > 1 {
                        println!(
                            "{}",
                            format!(
                                "📦 Detected split APK bundle ({} APKs). Installing splits...",
                                extracted_apks.len()
                            )
                            .cyan()
                            .bold()
                        );
                        return Self::install_splits(&extracted_apks, Some(&target_device));
                    } else if let Some(single_apk) = extracted_apks.first() {
                        return Self::install_single(single_apk, Some(&target_device));
                    }
                }
            }
        }

        Self::install_single(file_path, device_id)
    }

    /// Installs a single standalone APK via ADB
    pub fn install_single(apk_path: &Path, device_id: Option<&str>) -> Result<()> {
        let target_device = Self::resolve_device(device_id)?;
        println!(
            "{}",
            format!("📲 Installing on device '{}'...", target_device).cyan()
        );

        let status = Command::new("adb")
            .args(["-s", &target_device, "install", "-r", "-d"])
            .arg(apk_path)
            .status()
            .context(format!("Failed to install APK {:?}", apk_path))?;

        if status.success() {
            println!("{}", "✨ Successfully installed on device!".green().bold());
            Ok(())
        } else {
            Self::diagnose_install_failure(&target_device, Some(apk_path));
            Err(anyhow::anyhow!(
                "ADB install failed with exit code {:?}",
                status.code()
            ))
        }
    }

    /// Extracts native ABIs from a standalone APK
    pub fn get_apk_native_abis(apk_path: &Path) -> Vec<String> {
        let mut abis = Vec::new();
        if let Ok(file) = std::fs::File::open(apk_path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                for i in 0..archive.len() {
                    if let Ok(entry) = archive.by_index(i) {
                        let name = entry.name().to_lowercase();
                        if name.starts_with("lib/") {
                            let parts: Vec<&str> = name.split('/').collect();
                            if parts.len() >= 2 {
                                let abi = parts[1].to_string();
                                if !abis.contains(&abi) {
                                    abis.push(abi);
                                }
                            }
                        }
                    }
                }
            }
        }
        abis
    }

    /// Provides friendly troubleshooting advice based on Android package manager error codes
    pub fn diagnose_install_failure(target_device: &str, sample_apk: Option<&Path>) {
        if let Some(apk) = sample_apk {
            let apk_abis = Self::get_apk_native_abis(apk);
            let dev_abis = Self::get_device_abis(target_device);

            // Check for 32-bit vs 64-bit mismatch
            let is_32bit_only_apk = !apk_abis.is_empty()
                && apk_abis
                    .iter()
                    .all(|a| a == "armeabi-v7a" || a == "armeabi" || a == "x86");
            let is_64bit_only_dev =
                !dev_abis.is_empty() && dev_abis.iter().all(|a| a == "arm64-v8a" || a == "x86_64");

            if is_32bit_only_apk && is_64bit_only_dev {
                eprintln!(
                    "\n{}",
                    "💡 Architecture Incompatibility Detected:".yellow().bold()
                );
                eprintln!(
                    "  • APK compiled for : {} (32-bit legacy)",
                    apk_abis.join(", ").cyan()
                );
                eprintln!(
                    "  • Connected device : {} (64-bit-only hardware)",
                    dev_abis.join(", ").green()
                );
                eprintln!(
                    "\n👉 This older app release was compiled before 64-bit was standardized (pre-2019)."
                );
                eprintln!(
                    "👉 Modern Android phones (like your device) dropped 32-bit execution support."
                );
                eprintln!(
                    "👉 To install on this device, select a newer version (e.g. latest or post-2019) with 'gplay versions <package>'."
                );
            }
        }
    }

    /// Fetches the supported CPU architectures (ABIs) of the target device
    pub fn get_device_abis(device_id: &str) -> Vec<String> {
        let output = Command::new("adb")
            .args([
                "-s",
                device_id,
                "shell",
                "getprop",
                "ro.product.cpu.abilist",
            ])
            .output();

        let mut abis = Vec::new();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            for item in s.trim().split(',') {
                let trimmed = item.trim().to_lowercase();
                if !trimmed.is_empty() {
                    abis.push(trimmed);
                }
            }
        }

        if abis.is_empty() {
            if let Ok(out) = Command::new("adb")
                .args(["-s", device_id, "shell", "getprop", "ro.product.cpu.abi"])
                .output()
            {
                let s = String::from_utf8_lossy(&out.stdout);
                let trimmed = s.trim().to_lowercase();
                if !trimmed.is_empty() {
                    abis.push(trimmed);
                }
            }
        }

        abis
    }

    /// Filters split APKs to only include those matching the target device's supported ABIs
    pub fn filter_splits_for_device(split_paths: &[PathBuf], device_id: &str) -> Vec<PathBuf> {
        let device_abis = Self::get_device_abis(device_id);
        if device_abis.is_empty() {
            return split_paths.to_vec();
        }

        let known_abis = [
            "arm64_v8a",
            "arm64-v8a",
            "armeabi_v7a",
            "armeabi-v7a",
            "armeabi",
            "x86_64",
            "x86",
        ];

        let mut filtered = Vec::new();

        for path in split_paths {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_lowercase();

            let mut is_abi_split = false;
            let mut matches_device = false;

            for abi in &known_abis {
                if filename.contains(abi) {
                    is_abi_split = true;
                    let norm_abi = abi.replace('_', "-");
                    if device_abis
                        .iter()
                        .any(|d| d == &norm_abi || d.contains(&norm_abi))
                    {
                        matches_device = true;
                    }
                    break;
                }
            }

            if !is_abi_split || matches_device {
                filtered.push(path.clone());
            } else {
                println!(
                    "{}",
                    format!(
                        "ℹ️  Skipping non-matching ABI split '{}' for device ABIs ({})",
                        filename,
                        device_abis.join(", ")
                    )
                    .dimmed()
                );
            }
        }

        filtered
    }

    /// Installs multiple split APKs (base.apk + split_*.apk) via ADB install-multiple
    pub fn install_splits(split_paths: &[PathBuf], device_id: Option<&str>) -> Result<()> {
        if split_paths.is_empty() {
            return Err(anyhow::anyhow!("No split APKs provided to install"));
        }

        let target_device = Self::resolve_device(device_id)?;
        let compatible_splits = Self::filter_splits_for_device(split_paths, &target_device);

        println!(
            "{}",
            format!(
                "📲 Installing {} split APKs on device '{}' (adb install-multiple)...",
                compatible_splits.len(),
                target_device
            )
            .cyan()
        );

        let mut cmd = Command::new("adb");
        cmd.args(["-s", &target_device, "install-multiple", "-r", "-d"]);
        for path in &compatible_splits {
            cmd.arg(path);
        }

        let status = cmd.status().context("Failed to run adb install-multiple")?;

        if status.success() {
            println!(
                "{}",
                "✨ All split APKs successfully installed on device!"
                    .green()
                    .bold()
            );
            Ok(())
        } else {
            Self::diagnose_install_failure(
                &target_device,
                compatible_splits.first().map(|p| p.as_path()),
            );
            Err(anyhow::anyhow!(
                "ADB install-multiple failed with exit code {:?}",
                status.code()
            ))
        }
    }

    /// Installs an .apks bundle archive by extracting and running install-multiple
    pub fn install_apks(apks_path: &Path, device_id: Option<&str>) -> Result<()> {
        let file = std::fs::File::open(apks_path)
            .context(format!("Failed to open .apks bundle: {:?}", apks_path))?;
        let mut archive = ZipArchive::new(std::io::BufReader::new(file))?;

        let temp_dir = tempfile::tempdir()?;
        let mut split_paths = Vec::new();

        for i in 0..archive.len() {
            let mut zip_entry = archive.by_index(i)?;
            let name = zip_entry.name().to_string();
            if name.ends_with(".apk") {
                let out_path = temp_dir.path().join(&name);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut zip_entry, &mut out_file)?;
                split_paths.push(out_path);
            }
        }

        if split_paths.is_empty() {
            return Err(anyhow::anyhow!("No .apk files found inside .apks bundle"));
        }

        Self::install_splits(&split_paths, device_id)
    }
}
