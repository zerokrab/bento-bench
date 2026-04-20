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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a .tar.zst archive containing a valid suite structure,
    /// plus a flat variant (no wrapping directory).
    fn create_test_suite_archive(wrap_in_subdir: bool) -> (tempfile::TempDir, PathBuf) {
        let src_dir = tempfile::tempdir().unwrap();

        let content_dir = if wrap_in_subdir {
            let sub = src_dir.path().join("suite");
            std::fs::create_dir_all(&sub).unwrap();
            sub
        } else {
            src_dir.path().to_path_buf()
        };

        let images_dir = content_dir.join("images");
        let inputs_dir = content_dir.join("inputs");
        std::fs::create_dir_all(&images_dir).unwrap();
        std::fs::create_dir_all(&inputs_dir).unwrap();

        // Write manifest.json
        let manifest = serde_json::json!({
            "description": "test suite",
            "entries": []
        });
        std::fs::write(content_dir.join("manifest.json"), manifest.to_string()).unwrap();

        // Write dummy files
        std::fs::write(images_dir.join("test.elf"), b"elf-content").unwrap();
        std::fs::write(inputs_dir.join("test.input"), b"input-content").unwrap();

        // Create .tar.zst archive using CLI tools
        let archive_dir = tempfile::tempdir().unwrap();
        let archive_path = archive_dir.path().join("suite.tar.zst");

        let status = std::process::Command::new("tar")
            .arg("--zstd")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(src_dir.path())
            .arg(if wrap_in_subdir { "suite" } else { "." })
            .status()
            .expect("tar command failed to start");
        assert!(status.success(), "tar --zstd failed");

        (archive_dir, archive_path)
    }

    /// Extract a .tar.zst file using the same pipeline as fetch_suite
    /// (ZstdDecoder + tokio_tar), but reading from a local file.
    async fn extract_archive(archive_path: &PathBuf, dest: &std::path::Path) -> Result<()> {
        let file = tokio::fs::File::open(archive_path).await?;
        let buf_reader = tokio::io::BufReader::new(file);
        let zstd_reader = ZstdDecoder::new(buf_reader);
        let mut archive = tokio_tar::Archive::new(zstd_reader);
        archive.unpack(dest).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_extract_flat_archive() {
        let (_archive_dir, archive_path) = create_test_suite_archive(false);

        let extract_dir = tempfile::tempdir().unwrap();
        extract_archive(&archive_path, extract_dir.path())
            .await
            .expect("extraction failed");

        assert!(
            extract_dir.path().join("manifest.json").exists(),
            "manifest.json should be at top level for flat archive"
        );
    }

    #[tokio::test]
    async fn test_extract_subdirectory_archive() {
        let (_archive_dir, archive_path) = create_test_suite_archive(true);

        let extract_dir = tempfile::tempdir().unwrap();
        extract_archive(&archive_path, extract_dir.path())
            .await
            .expect("extraction failed");

        // The archive was created with `tar -C <src> suite`, so it contains
        // a `suite/` directory. Files extract under extract_dir/suite/.
        assert!(
            extract_dir.path().join("suite/manifest.json").exists(),
            "manifest.json should exist under suite/ subdirectory"
        );
    }

    #[tokio::test]
    async fn test_fetch_suite_rejects_bad_url() {
        let result = fetch_suite("http://127.0.0.1:1/nonexistent.tar.zst").await;
        assert!(result.is_err(), "expected error for bad URL");
    }
}
