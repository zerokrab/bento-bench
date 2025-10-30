use alloy::primitives::U256;
use anyhow::{Context, Result, bail};
use boundless_market::{
    Client, Deployment, contracts::RequestInputType, input::GuestEnv, storage::fetch_url,
};
use clap::Args;
use risc0_zkvm::compute_image_id;
use std::path::PathBuf;
use tracing::info;
use url::Url;

#[derive(Args, Clone, Debug)]
pub struct FetchAndSave {
    /// Proof request id to fetch.
    #[arg(long, value_delimiter = ',', required = true)]
    pub request_id: U256,
    #[clap(short = 'f', long)]
    pub file_name: Option<PathBuf>,
    /// RPC URL for the prover network
    #[clap(long = "rpc-url", env = "RPC_URL")]
    pub rpc_url: Option<Url>,
    /// Configuration for the Boundless deployment to use.
    #[clap(flatten, next_help_heading = "Boundless Deployment")]
    pub deployment: Option<Deployment>,
}

impl FetchAndSave {
    /// Run the benchmark command
    pub async fn run(&self) -> Result<()> {
        let client = Client::builder()
            .with_rpc_url(self.rpc_url.clone().context("Must specify RPC_URL")?)
            .with_deployment(self.deployment.clone())
            .with_timeout(None)
            .build()
            .await?;

        let request_id = self.request_id;
        info!("Fetching image and input for request 0x{request_id:x}");

        let (request, _signature) = client.fetch_proof_request(request_id, None, None).await?;
        info!(
            "Fetched request 0x{request_id:x} with image URL: {}",
            request.imageUrl
        );

        let elf = fetch_url(&request.imageUrl).await?;

        let input = match request.input.inputType {
            RequestInputType::Inline => GuestEnv::decode(&request.input.data)?.stdin,
            RequestInputType::Url => {
                let input_url = std::str::from_utf8(&request.input.data)
                    .context("Input URL is not valid UTF-8")?;
                tracing::debug!("Fetching input from {}", input_url);
                GuestEnv::decode(&fetch_url(input_url).await?)?.stdin
            }
            _ => bail!("Unsupported input type"),
        };
        let image_id = compute_image_id(&elf)?.to_string();
        info!("Image ID: {}", image_id);

        // Write the ELF and input to files if specified
        if let Some(file_name) = &self.file_name {
            let elf_file = file_name.with_extension("elf");
            let input_file = file_name.with_extension("input");
            std::fs::write(&elf_file, elf.clone()).context("Failed to write ELF file")?;
            std::fs::write(&input_file, input.clone()).context("Failed to write input file")?;
            info!("Wrote ELF to {:?} and input to {:?}", elf_file, input_file);
        }

        Ok(())
    }
}
