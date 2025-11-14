use crate::{DEFAULT_TASKDB_URL, config::ProverConfig};
use anyhow::{Context, Result, bail};
use bonsai_sdk::non_blocking::Client as BonsaiClient;
use clap::{Args, Parser, ValueEnum};
use core::IterReq;
use risc0_zkvm::{ExecutorEnv, ProverOpts, compute_image_id, default_prover, serde::to_vec};
use sqlx::{postgres::PgPool, postgres::PgPoolOptions};
use std::path::PathBuf;

/// Benchmark proof requests
#[derive(Args, Clone, Debug)]
pub struct ProverBenchmark {
    /// Proof request ids to benchmark.
    #[clap(short = 'f', long)]
    elf_file: Option<PathBuf>,

    /// ZKVM encoded input to be supplied to ExecEnv
    #[clap(short = 'i', long)]
    input_file: Option<PathBuf>,

    /// Run a execute only
    #[clap(short, long, default_value_t = false)]
    exec_only: bool,

    /// Prover configuration options
    #[clap(flatten, next_help_heading = "Prover")]
    prover_config: ProverConfig,

    /// Use the default prover instead of Bento
    #[clap(short = 'd', long)]
    default_prover: bool,

    #[clap(flatten, next_help_heading = "Built-in Program")]
    builtin_program: Option<BuiltInArgs>,
}

#[derive(Args, Debug, Clone)]
pub struct BuiltInArgs {
    /// Built-in program to run
    #[clap(short = 'p', long)]
    program: Option<BuiltInPrograms>,
}

#[derive(Parser, Debug, Clone)]
pub enum BuiltInPrograms {
    BentoIter {
        /// Number of iterations to run
        #[clap(short = 'c', long, default_value_t = 10)]
        iter_count: u64,
    },
}

impl ValueEnum for BuiltInPrograms {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::BentoIter { iter_count: 10 }]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::BentoIter { .. } => Some(clap::builder::PossibleValue::new("bento_iter")),
        }
    }
}

impl ProverBenchmark {
    /// Run the benchmark command
    pub async fn run(&self) -> Result<()> {
        let (elf, input) = if let Some(program) = self.builtin_program.as_ref() {
            match program.program {
                Some(BuiltInPrograms::BentoIter { iter_count }) => {
                    tracing::info!("Using built-in BentoIter with {} iterations", iter_count);
                    let input = to_vec(&IterReq::Iter(iter_count)).expect("Failed to r0 to_vec");
                    let input = bytemuck::cast_slice(&input).to_vec();
                    (guests::BENTO_SAMPLE_ELF.to_vec(), input)
                }
                None => bail!("No built-in program specified"),
            }
        } else {
            load_elf_and_input(&self.elf_file, &self.input_file)?
        };

        let image_id = compute_image_id(&elf)?.to_string();

        let (total_cycles, elapsed_secs) = if self.default_prover {
            tracing::info!("Using default prover!");
            let env = ExecutorEnv::builder().write_slice(&input).build()?;
            let prover = default_prover();
            let start_time: std::time::Instant = std::time::Instant::now();

            let res = prover.prove_with_opts(env, &elf, &ProverOpts::succinct())?;
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            (res.stats.total_cycles as f64, elapsed_secs)
        } else {
            self.prover_config
                .proving_backend
                .configure_proving_backend();

            let prover = BonsaiClient::from_env(risc0_zkvm::VERSION)?;

            // Check if we can connect to PostgreSQL using environment variables
            let pg_pool = match create_pg_pool().await {
                Ok(pool) => {
                    tracing::info!("Connected to PostgreSQL database for enhanced metrics.");

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
                .create_session(image_id, input_id, assumptions.clone(), self.exec_only)
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
                tracing::debug!("No PostgreSQL data found for job, using client-side calculation.");
                let total_cycles: f64 = stats.total_cycles as f64;
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                (total_cycles, elapsed_secs)
            }
        };

        let effective_khz = total_cycles / elapsed_secs / 1000.0;

        tracing::info!(
            "Proof completed: {:.0} cycles in {:.2} seconds ({:.2} KHz)",
            total_cycles,
            elapsed_secs,
            effective_khz
        );
        Ok(())
    }
}

fn load_elf_and_input(
    elf_path: &Option<PathBuf>,
    input_path: &Option<PathBuf>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let elf = match elf_path {
        Some(path) => std::fs::read(path).context("Failed to read ELF file")?,
        None => bail!("ELF file path must be provided"),
    };

    let input = match input_path {
        Some(path) => std::fs::read(path).context("Failed to read input file")?,
        None => bail!("Input file path must be provided"),
    };

    Ok((elf, input))
}

/// Create a PostgreSQL connection pool from environment variables
pub async fn create_pg_pool() -> Result<PgPool> {
    let database_url = if let Ok(url) = std::env::var("DATABASE_URL") {
        url
    } else {
        tracing::info!(
            "DATABASE_URL not set; attempting to use DEFAULT_TASKDB_URL: {}",
            DEFAULT_TASKDB_URL
        );
        DEFAULT_TASKDB_URL.to_string()
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to PostgreSQL database")?;

    Ok(pool)
}
