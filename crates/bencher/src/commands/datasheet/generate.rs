use crate::commands::datasheet::config::DatasheetConfig;
use crate::datasheet::DatasheetEntry;
use crate::datasheet::Manifest;
use crate::prove::prove_bonsai;
use anyhow::{Context, Result};
use bonsai_sdk::non_blocking::Client as BonsaiClient;
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    #[clap(short, long, default_value_t = true)]
    exec_only: bool,
}

impl GenerateArgs {
    /// Run the datasheet generate command
    pub async fn run(&self, config: DatasheetConfig) -> Result<()> {
        let manifest = std::fs::read_to_string(config.archive_dir.join("manifest.json"))
            .context("Failed to read manifest file")?;
        let manifest: Manifest =
            serde_json::from_str(&manifest).context("Failed to parse manifest JSON")?;

        config
            .prover_config
            .proving_backend
            .configure_proving_backend();

        // Currently only support Bento, not default prover
        let prover: BonsaiClient = BonsaiClient::from_env(risc0_zkvm::VERSION)?;

        let mut res = Vec::new();
        for (label, entries) in manifest.0.iter() {
            for entry in entries.iter() {
                let image_id = entry.image_id.clone().expect("image id missing");
                let elf = std::fs::read(
                    config
                        .archive_dir
                        .join("images")
                        .join(format!("{}.elf", image_id)),
                )?;
                let input = std::fs::read(
                    config
                        .archive_dir
                        .join("inputs")
                        .join(format!("0x{:x}.input", entry.request_id)),
                )?;

                let (total_cycles, elapsed_secs) =
                    prove_bonsai(prover.clone(), image_id, elf, input, self.exec_only).await?;

                res.push(DatasheetEntry {
                    label: label.clone(),
                    uuid: entry.uuid.expect("uuid missing"),
                    description: entry.description.clone(),
                    num_cycles: total_cycles,
                    elapsed_time_secs: elapsed_secs,
                });
            }
        }
        let output_path = config.archive_dir.join("datasheet.json");
        std::fs::create_dir_all(&config.archive_dir)
            .context("Failed to create output directory")?;
        std::fs::write(
            &output_path,
            serde_json::to_string_pretty(&res).context("Failed to serialize datasheet to JSON")?,
        )
        .context("Failed to write datasheet file")?;
        tracing::info!("Wrote datasheet to {:?}", output_path);

        Ok(())
    }
}
