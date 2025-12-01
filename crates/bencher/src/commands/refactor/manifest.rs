use alloy::primitives::U256;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::write;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub description: String,
    pub entries: Vec<ManifestEntryV2>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestEntryV2 { // TODO: Rename once other impl is removed
    /// Description of the request
    pub description: String,
    /// Proof request id to fetch.
    pub request_id: Option<U256>,
    pub input_id: Option<String>,
    pub image_id: Option<String>,
}

// TODO: Handle creating manifest if does not exist
pub fn load_manifest(manifest_path: &Path) -> Result<Manifest> {
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest file: {:?}", manifest_path))?;
    serde_json::from_str(&manifest_str)
        .with_context(|| format!("Failed to parse manifest file: {:?}", manifest_path))
}

pub async fn write_manifest(manifest: &Manifest, output_path: &Path) -> Result<()> {
    let out_str = serde_json::to_string_pretty(&manifest)?;
    write(&output_path, out_str).await.context(format!(
        "Failed to write manifest file to {:?}",
        output_path
    ))
}
