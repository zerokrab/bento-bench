use crate::commands::bench::create_pg_pool;
use crate::commands::{NetworkArgs, prepare::Manifest};
use crate::{DEFAULT_TASKDB_URL, config::ProverConfig};
use alloy::primitives::U256;
use anyhow::{Context, Result, bail, ensure};
use bonsai_sdk::non_blocking::Client as BonsaiClient;
use boundless_market::{Client, contracts::RequestInputType, input::GuestEnv, storage::fetch_url};
use clap::{Args, Subcommand};
use risc0_zkvm::{ExecutorEnv, ProverOpts, compute_image_id, default_prover, serde::to_vec};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
#[derive(Args, Clone, Debug)]
pub struct DatasheetArgs {
    #[command(subcommand)]
    pub command: Datasheet,
    /// Prover configuration options
    #[clap(flatten, next_help_heading = "Prover")]
    prover_config: ProverConfig,
    /// Use the default prover instead of Bento
    #[clap(short = 'd', long)]
    default_prover: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Datasheet {
    Generate(GenerateArgs),
}
#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    /// RPC URL for the prover network
    #[clap(long, short = 'm')]
    archive_dir: PathBuf,
    #[clap(long, short = 'o')]
    output_dir: PathBuf,
    /// Run a execute only
    #[clap(short, long, default_value_t = true)]
    exec_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasheetEntry {
    pub label: String,
    pub uuid: uuid::Uuid,
    pub description: String,
    pub num_cycles: f64,
    pub elapsed_time_secs: f64,
}

impl DatasheetArgs {
    /// Run the datasheet command
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Datasheet::Generate(args) => {
                self.generate(args).await?;
            }
        }
        Ok(())
    }
    async fn generate(&self, args: &GenerateArgs) -> Result<()> {
        let manifest = std::fs::read_to_string(args.archive_dir.join("manifest.json"))
            .context("Failed to read manifest file")?;
        let manifest: Manifest =
            serde_json::from_str(&manifest).context("Failed to parse manifest JSON")?;
        let mut res = Vec::new();
        for (label, entries) in manifest.0.iter() {
            for entry in entries.iter() {
                let image_id = entry.image_id.clone().expect("image id missing");
                let elf = std::fs::read(
                    args.archive_dir
                        .join("images")
                        .join(format!("{}.elf", image_id)),
                )?;
                let input = std::fs::read(
                    args.archive_dir
                        .join("inputs")
                        .join(format!("0x{:x}.input", entry.request_id)),
                )?;

                let (total_cycles, elapsed_secs) = if self.default_prover {
                    todo!()
                } else {
                    self.prover_config
                        .proving_backend
                        .configure_proving_backend();

                    let prover = BonsaiClient::from_env(risc0_zkvm::VERSION)?;

                    // Check if we can connect to PostgreSQL using environment variables
                    let pg_pool = match create_pg_pool().await {
                        Ok(pool) => {
                            tracing::info!(
                                "Connected to PostgreSQL database for enhanced metrics."
                            );

                            Some(pool)
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Failed to connect to PostgreSQL database: {}, no enhanced metrics.",
                                e
                            );
                            None
                        }
                    };

                    prover.upload_img(&image_id, elf).await.unwrap();
                    tracing::info!("Uploaded ELF to {}", image_id);

                    let input_id = prover
                        .upload_input(input)
                        .await
                        .context("Failed to upload set-builder input")?;
                    tracing::info!("Uploaded input to {}", input_id);

                    let assumptions = vec![];
                    let start_time = std::time::Instant::now();

                    let proof_id = prover
                        .create_session(image_id, input_id, assumptions.clone(), args.exec_only)
                        .await?;
                    tracing::info!("Created session {}", proof_id.uuid);

                    let (stats, _elapsed_time) = loop {
                        let status = proof_id.status(&prover).await?;

                        match status.status.as_ref() {
                            "RUNNING" => {
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                continue;
                            }
                            "SUCCEEDED" => {
                                let Some(stats) = status.stats else {
                                    bail!("Bento failed to return proof stats in response");
                                };
                                break (stats, status.elapsed_time);
                            }
                            _ => {
                                let err_msg = status.error_msg.unwrap_or_default();
                                bail!("stark proving failed: {err_msg}");
                            }
                        }
                    };

                    // Try to get effective KHz from PostgreSQL if available
                    if let Some(ref pool) = pg_pool {
                        let total_cycles_query = r#"
                    SELECT (output->>'total_cycles')::FLOAT8
                    FROM tasks
                    WHERE task_id = 'init' AND job_id = $1::uuid
                "#;

                        let elapsed_secs_query = r#"
                    SELECT EXTRACT(EPOCH FROM (MAX(updated_at) - MIN(started_at)))::FLOAT8
                    FROM tasks
                    WHERE job_id = $1::uuid
                "#;

                        let cycles_result = sqlx::query_scalar::<_, f64>(total_cycles_query)
                            .bind(proof_id.uuid.clone())
                            .fetch_optional(pool)
                            .await;

                        let elapsed_result = sqlx::query_scalar::<_, f64>(elapsed_secs_query)
                            .bind(proof_id.uuid.clone())
                            .fetch_optional(pool)
                            .await;

                        match (cycles_result, elapsed_result) {
                            (Ok(Some(cycles)), Ok(Some(elapsed))) => {
                                tracing::debug!(
                                    "Retrieved from PostgreSQL: {} cycles in {} seconds",
                                    cycles,
                                    elapsed
                                );
                                (cycles, elapsed)
                            }
                            _ => {
                                tracing::debug!(
                                    "Failed to retrieve data from PostgreSQL, using client-side calculation"
                                );
                                let total_cycles: f64 = stats.total_cycles as f64;
                                let elapsed_secs = start_time.elapsed().as_secs_f64();
                                (total_cycles, elapsed_secs)
                            }
                        }
                    } else {
                        tracing::debug!(
                            "No PostgreSQL data found for job, using client-side calculation."
                        );
                        let total_cycles: f64 = stats.total_cycles as f64;
                        let elapsed_secs = start_time.elapsed().as_secs_f64();
                        (total_cycles, elapsed_secs)
                    }
                };
                res.push(DatasheetEntry {
                    label: label.clone(),
                    uuid: entry.uuid.expect("uuid missing"),
                    description: entry.description.clone(),
                    num_cycles: total_cycles,
                    elapsed_time_secs: elapsed_secs,
                });
            }
        }
        let output_path = args.output_dir.join("datasheet.json");
        std::fs::create_dir_all(&args.output_dir).context("Failed to create output directory")?;
        std::fs::write(
            &output_path,
            serde_json::to_string_pretty(&res).context("Failed to serialize datasheet to JSON")?,
        )
        .context("Failed to write datasheet file")?;
        tracing::info!("Wrote datasheet to {:?}", output_path);

        Ok(())
    }
}
