use crate::commands::CommonArgs;
use crate::commands::manifest::{ManifestEntry, load_manifest, write_manifest};
use crate::commands::prepare::{compute_cycles, fetch_image, fetch_input};
use alloy::primitives::{TxHash, U256};
use anyhow::{Context, Result};
use boundless_market::Client;
use clap::Args;
use tokio::fs::create_dir_all;
use url::Url;

#[derive(Args, Clone, Debug)]
pub struct PrepareRequestArgs {
    #[clap(flatten)]
    common: CommonArgs,
    /// Request ID to fetch
    #[clap(long)]
    request_id: U256,
    /// Tx hash from request submission
    #[clap(long)]
    tx_hash: Option<TxHash>,
    /// Description of the request/image/input
    #[clap(long)]
    description: String,
    /// RPC endpoint to query the request info
    #[clap(long, env)]
    rpc_url: Url,
}

impl PrepareRequestArgs {
    pub async fn run(&self) -> Result<()> {
        let data_dir = self.common.data_dir.clone();

        let mut manifest = load_manifest(&self.common.data_dir)?;

        let images_dir = data_dir.join("images");
        create_dir_all(&images_dir).await.context(format!(
            "Failed to create images directory: {}",
            images_dir.display()
        ))?;

        let inputs_dir = data_dir.join("inputs");
        create_dir_all(&inputs_dir).await.context(format!(
            "Failed to create inputs directory: {}",
            inputs_dir.display()
        ))?;

        tracing::info!("Fetching data for request 0x{:x}", &self.request_id);
        let client = Client::builder()
            .with_rpc_url(self.rpc_url.clone())
            .with_timeout(None)
            .build()
            .await?;

        let (request, _signature) = client
            .fetch_proof_request(self.request_id, self.tx_hash, None, None, None)
            .await?;
        tracing::info!("Fetching image...");
        let image_id = fetch_image(&request.imageUrl, &images_dir).await?;
        tracing::info!("Fetching input...");
        let input_id = fetch_input(&request, &inputs_dir).await?;

        let image_path = images_dir.join(format!("{image_id}.elf"));
        let input_path = inputs_dir.join(format!("{input_id}.input"));

        tracing::info!("Running executor to determine cycle count...");
        let cycles = compute_cycles(&input_path, &image_path).await?;

        let entry = ManifestEntry {
            image_id: Some(image_id),
            input_id: Some(input_id),
            description: self.description.clone(),
            cycles,
        };

        manifest.entries.push(entry);

        write_manifest(&manifest, &self.common.data_dir).await?;

        Ok(())
    }
}
