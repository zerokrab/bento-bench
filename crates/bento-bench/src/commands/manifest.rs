use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tokio::fs::write;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub description: String,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Description of the request
    pub description: String,
    /// Hash of input
    pub input_id: Option<String>,
    /// Image ID
    pub image_id: Option<String>,
    /// Exec cycle count
    pub cycles: u64,
}

pub fn load_manifest(manifest_dir: &Path) -> Result<Manifest> {
    let manifest_path = manifest_dir.join("manifest.json");

    if !fs::exists(&manifest_path).unwrap_or(false) {
        let manifest = Manifest {
            description: String::from("TODO"),
            entries: Vec::new(),
        };
        tracing::warn!("New manifest file will be created, description needs to be updated");
        return Ok(manifest);
    }

    let manifest_str = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest file: {manifest_path:?}"))?;
    serde_json::from_str(&manifest_str)
        .with_context(|| format!("Failed to parse manifest file: {manifest_path:?}"))
}

pub async fn write_manifest(manifest: &Manifest, output_dir: &Path) -> Result<()> {
    let output_path = output_dir.join("manifest.json");
    let out_str = serde_json::to_string_pretty(&manifest)?;
    write(&output_path, out_str)
        .await
        .context(format!("Failed to write manifest file to {output_path:?}",))
}
