use crate::models::{AppDelivery, PlayDownloadFile};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Download all files in an AppDelivery payload into the target directory
    pub async fn download_delivery(
        &self,
        delivery: &AppDelivery,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        tokio::fs::create_dir_all(output_dir).await?;

        let mp = MultiProgress::new();
        let mut files_to_download = vec![delivery.base_file.clone()];
        files_to_download.extend(delivery.split_files.clone());

        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
            .context("Invalid progress bar template")?
            .progress_chars("#>-");

        let mut downloaded_paths = Vec::new();

        for file_info in files_to_download {
            let target_path = output_dir.join(&file_info.name);
            let pb = mp.add(ProgressBar::new(file_info.size_bytes));
            pb.set_style(style.clone());
            pb.set_message(format!("Downloading {}", file_info.name));

            self.download_single_file(&file_info, &target_path, &pb)
                .await?;
            pb.finish_with_message(format!("Downloaded {}", file_info.name));
            downloaded_paths.push(target_path);
        }

        Ok(downloaded_paths)
    }

    async fn download_single_file(
        &self,
        file_info: &PlayDownloadFile,
        target_path: &Path,
        pb: &ProgressBar,
    ) -> Result<()> {
        let response = self
            .client
            .get(&file_info.download_url)
            .send()
            .await
            .context(format!("Failed to request {}", file_info.name))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download for {} returned HTTP {}",
                file_info.name,
                response.status()
            ));
        }

        if let Some(content_length) = response.content_length() {
            if content_length > 0 {
                pb.set_length(content_length);
            }
        }

        let mut file = File::create(target_path).await?;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Error while streaming download chunks")?;
            file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }

        file.flush().await?;
        Ok(())
    }
}
