use crate::commands::NetworkArgs;
use alloy::primitives::U256;
use anyhow::{Context, Result, bail, ensure};
use boundless_market::{Client, contracts::RequestInputType, input::GuestEnv, storage::fetch_url};
use clap::{Args, Subcommand};
use risc0_zkvm::compute_image_id;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::fs::create_dir_all;

#[derive(Args, Clone, Debug)]
pub struct PrepareArgs {
    #[command(subcommand)]
    pub command: Prepare,
    #[clap(flatten, next_help_heading = "Boundless Deployment")]
    pub network: NetworkArgs,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Prepare {
    Generate(GenerateArgs),
}
#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    /// RPC URL for the prover network
    #[clap(long, short = 'm')]
    manifest: PathBuf,
    #[clap(long, short = 'a')]
    archive_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Manifest(
    /// Names of programs to benchmark
    HashMap<String, Vec<ManifestEntry>>,
);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ManifestEntry {
    /// Will be filled in if not provided by fetching the request
    pub image_id: Option<String>,
    /// Proof request id to fetch.
    pub request_id: U256,
    /// Description of the request
    pub description: String,
}

impl PrepareArgs {
    /// Run the prepare command
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Prepare::Generate(args) => {
                self.generate(args).await?;
            }
        }
        Ok(())
    }

    async fn generate(&self, args: &GenerateArgs) -> Result<()> {
        let client = Client::builder()
            .with_rpc_url(
                self.network
                    .rpc_url
                    .clone()
                    .context("Must specify RPC_URL")?,
            )
            .with_deployment(self.network.deployment.clone())
            .with_timeout(None)
            .build()
            .await?;

        let manifest_str = std::fs::read_to_string(&args.manifest)
            .with_context(|| format!("Failed to read manifest file: {:?}", args.manifest))?;

        let mut manifest: Manifest = serde_json::from_str(&manifest_str)
            .with_context(|| format!("Failed to parse manifest file: {:?}", args.manifest))?;
        let images_dir = args.archive_dir.join("images");
        create_dir_all(&images_dir).await.context(format!(
            "Failed to create images directory: {:?}",
            images_dir
        ))?;

        let inputs_dir = args.archive_dir.join("inputs");
        create_dir_all(&inputs_dir).await.context(format!(
            "Failed to create inputs directory: {:?}",
            inputs_dir
        ))?;

        for (label, entries) in manifest.0.iter_mut() {
            for entry in entries.iter_mut() {
                let (request, _signature) = client
                    .fetch_proof_request(entry.request_id, None, None)
                    .await?;
                let mut fetch_and_save_image = false;
                match &entry.image_id {
                    Some(image_id) => {
                        let elf_path = images_dir.join(format!("{}.elf", image_id));
                        if !does_file_exist(&elf_path).await {
                            tracing::info!(
                                "Image ID specified but ELF file does not exist, fetching ELF for request ID: 0x{:x}",
                                entry.request_id
                            );
                            fetch_and_save_image = true;
                        } else {
                            tracing::debug!("ELF file exists in archive dir: {:?}", elf_path);
                            ensure!(
                                &compute_image_id(&std::fs::read(&elf_path)?)?.to_string()
                                    == image_id,
                                "Image ID mismatch for entry: {}",
                                label
                            );
                        }
                    }
                    None => {
                        tracing::info!(
                            "No image_id specified, fetching ELF for request ID: 0x{:x}",
                            entry.request_id
                        );
                        fetch_and_save_image = true;
                    }
                }
                if fetch_and_save_image {
                    let elf = fetch_url(&request.imageUrl).await?;
                    let computed_image_id = compute_image_id(&elf)?.to_string();
                    entry.image_id = Some(computed_image_id.clone());
                    tracing::info!("Computed Image ID: {}", computed_image_id);
                    let elf_path = images_dir.join(format!("{}.elf", computed_image_id));
                    // Write the ELF to the archive dir if it doesn't exist
                    if !does_file_exist(&elf_path).await {
                        tracing::info!("Writing ELF to images dir: {:?}", elf_path);
                        std::fs::write(&elf_path, elf).with_context(|| {
                            format!("Failed to write ELF file to {:?}", elf_path)
                        })?;
                    }
                }
                let input_path = inputs_dir.join(format!("0x{:x}.input", entry.request_id));

                if !does_file_exist(&input_path).await {
                    tracing::info!("Fetching input for request ID: 0x{:x}", entry.request_id);
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
                    std::fs::write(&input_path, input).with_context(|| {
                        format!("Failed to write input file to {:?}", input_path)
                    })?;
                }
            }
        }
        let manifest_json =
            serde_json::to_vec_pretty(&manifest).context("Failed to serialize manifest to JSON")?;
        std::fs::write(args.archive_dir.join("manifest.json"), manifest_json)
            .context("Failed to write updated manifest file")?;

        Ok(())
    }
}

async fn does_file_exist(path: &PathBuf) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}
