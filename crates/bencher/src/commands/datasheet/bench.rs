use crate::commands::datasheet::config::DatasheetConfig;
use crate::datasheet::DatasheetEntry;
use crate::datasheet::Manifest;
use crate::datasheet::db::DatasheetDb;
use crate::prove::prove_bonsai;
use anyhow::{Context, Result};
use bonsai_sdk::non_blocking::Client as BonsaiClient;
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct BenchArgs {
    #[clap(short, long, default_value_t = true)]
    exec_only: bool,
    #[clap(long)]
    manifest_uuid: Option<uuid::Uuid>,
}

impl BenchArgs {
    /// Run the datasheet generate command
    pub async fn run(&self, config: DatasheetConfig) -> Result<()> {
        let db_path = config.archive_dir.join("datasheet.db");
        let db = DatasheetDb::new(&db_path)
            .await
            .context("Failed to initialize SQLite database")?;

        let manifest: Manifest = if let Some(uuid) = self.manifest_uuid {
            db.get_manifest_by_uuid(uuid)
                .await
                .context("Failed to fetch manifest by UUID from database")?
        } else {
            tracing::info!("Generating datasheet for latest manifest");
            let latest_uuid = db
                .get_latest_manifest()
                .await
                .context("Failed to fetch latest manifest UUID from database")?;
            db.get_manifest_by_uuid(latest_uuid)
                .await
                .context("Failed to fetch latest manifest from database")?
        };
        tracing::info!(
            "Generating datasheet for manifest UUID: {}",
            manifest.id.unwrap()
        );

        config
            .prover_config
            .proving_backend
            .configure_proving_backend();

        // Currently only support Bento, not default prover
        let prover: BonsaiClient = BonsaiClient::from_env(risc0_zkvm::VERSION)?;

        let mut res = Vec::new();
        for entry in manifest.entries.iter() {
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

            let (session_stats, exec_elapsed_secs) = prove_bonsai(
                prover.clone(),
                image_id.clone(),
                elf.clone(),
                input.clone(),
                true,
            )
            .await?;

            let prove_elapsed_secs = if self.exec_only {
                0.0
            } else {
                prove_bonsai(
                    prover.clone(),
                    image_id.clone(),
                    elf.clone(),
                    input.clone(),
                    self.exec_only,
                )
                .await?
                .1
            };

            res.push(DatasheetEntry {
                id: None,
                manifest_entry_id: entry.id.expect("manifest entry id missing"),
                label: entry.label.clone(),
                description: entry.description.clone(),
                segments: session_stats.segments as u64,
                total_cycles: session_stats.total_cycles,
                cycles: session_stats.cycles,
                exec_time_secs: exec_elapsed_secs,
                prove_time_secs: prove_elapsed_secs,
            });
        }

        let mut datasheet = crate::datasheet::Datasheet {
            id: None,
            manifest_id: manifest.id.expect("manifest id missing"),
            entries: res,
        };
        db.insert_datasheet(&mut datasheet)
            .await
            .context("Failed to insert datasheet into database")?;
        tracing::info!(
            "Inserted datasheet with ID {:?} into database",
            datasheet.id
        );
        let output_path = config
            .archive_dir
            .join(format!("datasheet-{}.json", datasheet.id.unwrap()));
        std::fs::create_dir_all(&config.archive_dir)
            .context("Failed to create output directory")?;
        std::fs::write(
            &output_path,
            serde_json::to_string_pretty(&datasheet)
                .context("Failed to serialize datasheet to JSON")?,
        )
        .context("Failed to write datasheet file")?;
        tracing::info!("Wrote datasheet to {:?}", output_path);
        Ok(())
    }
}
