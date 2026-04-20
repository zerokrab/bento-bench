use anyhow::{Context, Result, bail};
use async_compression::tokio::bufread::ZstdDecoder;
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::io::BufReader;
use tokio_util::io::StreamReader;

/// Download a `.tar.zst` suite from `url` and extract it into a temp directory.
/// Returns the path to the extracted data directory.
pub async fn fetch_suite(url: &str) -> Result<PathBuf> {
    tracing::info!("Fetching suite from {url}");

    // Create a temp directory under the system temp dir
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let data_dir = temp_dir.path().to_path_buf();

    // Download the archive with streaming
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to fetch suite from {url}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Failed to fetch suite: HTTP {status}: {body}");
    }

    // Stream the response body through zstd decompression then tar extraction
    let stream = response
        .bytes_stream()
        .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let reader = StreamReader::new(stream);
    let buf_reader = BufReader::new(reader);
    let zstd_reader = ZstdDecoder::new(buf_reader);
    let mut archive = tokio_tar::Archive::new(zstd_reader);
    archive
        .unpack(&data_dir)
        .await
        .context("Failed to extract suite archive")?;

    // Check if the archive extracted into a subdirectory (common for tar archives).
    // If the data dir contains a single subdirectory and no manifest.json at top level,
    // use that subdirectory as the data dir instead.
    let effective_dir = if !data_dir.join("manifest.json").exists() {
        let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&data_dir)
            .context("Failed to read extracted directory")?
            .filter_map(|e| e.ok())
            .collect();

        if entries.len() == 1
            && entries[0].path().is_dir()
            && entries[0].path().join("manifest.json").exists()
        {
            entries.remove(0).path()
        } else {
            data_dir.clone()
        }
    } else {
        data_dir.clone()
    };

    // Validate essential files exist
    if !effective_dir.join("manifest.json").exists() {
        bail!("Fetched suite does not contain manifest.json");
    }

    tracing::info!("Suite extracted to {}", effective_dir.display());
    Ok(effective_dir)
}
