use crate::datasheet::db::DatasheetDb;
use crate::{commands::datasheet::config::DatasheetConfig, datasheet::Manifest};
use anyhow::{Context, Result, bail, ensure};
use boundless_market::{Client, contracts::RequestInputType, input::GuestEnv, storage::fetch_url};
use clap::Args;
use risc0_zkvm::compute_image_id;
use std::path::PathBuf;
use tokio::fs::create_dir_all;
#[derive(Args, Clone, Debug)]
pub struct PrepareArgs {}

impl PrepareArgs {
    pub(crate) async fn run(&self, config: DatasheetConfig) -> Result<()> {
        let db_path = config.archive_dir.join("datasheet.db");
        let db = DatasheetDb::new(&db_path)
            .await
            .context("Failed to initialize SQLite database")?;

        let prover_config = config.prover_config.clone();

        let archive_dir = config.archive_dir.clone();
        let manifest_path = config
            .manifest_path
            .clone()
            .unwrap_or_else(|| archive_dir.join("manifest.json"));

        let client = Client::builder()
            .with_rpc_url(
                prover_config
                    .rpc_url
                    .clone()
                    .context("Must specify RPC_URL")?,
            )
            .with_deployment(prover_config.deployment.clone())
            .with_timeout(None)
            .build()
            .await?;

        let manifest_str = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read manifest file: {:?}", manifest_path))?;

        let mut manifest: Manifest = serde_json::from_str(&manifest_str)
            .with_context(|| format!("Failed to parse manifest file: {:?}", manifest_path))?;
        let images_dir = archive_dir.join("images");
        create_dir_all(&images_dir).await.context(format!(
            "Failed to create images directory: {:?}",
            images_dir
        ))?;

        let inputs_dir = archive_dir.join("inputs");
        create_dir_all(&inputs_dir).await.context(format!(
            "Failed to create inputs directory: {:?}",
            inputs_dir
        ))?;

        for entry in manifest.entries.iter_mut() {
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
                            &compute_image_id(&std::fs::read(&elf_path)?)?.to_string() == image_id,
                            "Image ID mismatch for entry: {}",
                            entry.label
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
                    std::fs::write(&elf_path, elf)
                        .with_context(|| format!("Failed to write ELF file to {:?}", elf_path))?;
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
                std::fs::write(&input_path, input)
                    .with_context(|| format!("Failed to write input file to {:?}", input_path))?;
            }
        }
        let manifest_uuid = db
            .insert_manifest(&mut manifest)
            .await
            .context("Failed to insert manifest into database")?;
        tracing::info!("Inserted manifest with ID {} into database", manifest_uuid);

        let manifest_json =
            serde_json::to_vec_pretty(&manifest).context("Failed to serialize manifest to JSON")?;
        let manifest_path = archive_dir.join(format!("manifest-{}.json", manifest_uuid));
        std::fs::write(&manifest_path, &manifest_json)
            .context("Failed to write updated manifest file")?;
        Ok(())
    }
}

async fn does_file_exist(path: &PathBuf) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}
