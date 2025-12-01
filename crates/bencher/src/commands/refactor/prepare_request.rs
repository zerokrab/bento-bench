use crate::commands::refactor::prepare::{fetch_image, fetch_input};
use crate::commands::refactor::{Manifest, ManifestEntryV2};
use alloy::primitives::{U256};
use anyhow::{Context, Result};
use boundless_market::{Client};
use clap::Args;
use std::path::{PathBuf};
use tokio::fs;
use tokio::fs::create_dir_all;
use url::Url;

#[derive(Args, Clone, Debug)]
pub struct PrepareRequestArgs {
    #[clap(long = "manifest", default_value = "./manifest.json")]
    manifest_path: String,
    #[clap(long)]
    request_id: U256,
    #[clap(long)]
    description: String,
    #[clap(long)]
    data_dir: PathBuf,
    #[clap(long, env)]
    rpc_url: Url,
}

impl PrepareRequestArgs {
    pub(crate) async fn run(&self) -> Result<()> {
        let data_dir = self.data_dir.clone();
        let manifest_path = self.manifest_path.clone();

        let manifest_str = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read manifest file: {:?}", manifest_path))?;
        let mut manifest: Manifest = serde_json::from_str(&manifest_str)
            .with_context(|| format!("Failed to parse manifest file: {:?}", manifest_path))?;

        let images_dir = data_dir.join("images");
        create_dir_all(&images_dir).await.context(format!(
            "Failed to create images directory: {:?}",
            images_dir
        ))?;

        let inputs_dir = data_dir.join("inputs");
        create_dir_all(&inputs_dir).await.context(format!(
            "Failed to create inputs directory: {:?}",
            inputs_dir
        ))?;

        tracing::info!("Fetching data for request ID: {}", &self.request_id);
        let client = Client::builder()
            .with_rpc_url(self.rpc_url.clone())
            .with_timeout(None)
            .build()
            .await?;

        let (request, _signature) = client
            .fetch_proof_request(self.request_id.clone(), None, None)
            .await?;

        let image_id = fetch_image(&request.imageUrl, &images_dir).await?;
        let input_id = fetch_input(&request, &inputs_dir).await?;

        let entry = ManifestEntryV2 {
            image_id: Some(image_id),
            input_id: Some(input_id),
            description: self.description.clone(),
            request_id: Some(self.request_id),
        };

        manifest.entries.push(entry);

        let out_str = serde_json::to_string_pretty(&manifest)?;
        fs::write(&self.manifest_path, out_str)
            .await
            .context(format!(
                "Failed to write manifest file to {:?}",
                self.manifest_path
            ))?;

        Ok(())
    }
}
