use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub struct ApksBundler;

impl ApksBundler {
    /// Packages base.apk and split APKs into a standard .apks ZIP archive
    pub fn create_bundle(split_paths: &[PathBuf], output_apks_path: &Path) -> Result<()> {
        if let Some(parent) = output_apks_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let out_file = File::create(output_apks_path)
            .context(format!("Failed to create bundle: {:?}", output_apks_path))?;
        let mut zip = ZipWriter::new(BufWriter::new(out_file));

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored) // Keep APKs uncompressed inside .apks
            .unix_permissions(0o644);

        for path in split_paths {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("entry.apk");

            zip.start_file(filename, options)?;
            let mut file = BufReader::new(File::open(path)?);
            let mut buffer = [0u8; 65536];
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                zip.write_all(&buffer[..n])?;
            }
        }

        zip.finish()?;
        Ok(())
    }
}
