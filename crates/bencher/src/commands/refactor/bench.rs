use crate::ProverConfig;
use crate::commands::refactor::CommonArgs;
use crate::commands::refactor::manifest::load_manifest;
use crate::prove::prove_bonsai;
use anyhow::{Context, Result};
use bonsai_sdk::non_blocking::Client as BonsaiClient;
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;
use tabled::settings::{Reverse, Rotate, Settings, Style};
use tabled::{Table, Tabled};
use tokio::fs::write;

#[derive(Args, Clone, Debug)]
pub struct BenchArgs {
    #[clap(flatten)]
    common: CommonArgs,
    /// Execute only (no prove)
    #[clap(long, default_value_t = false)]
    exec_only: bool,
    /// Output results to json file
    #[clap(long)]
    json: Option<PathBuf>,

    #[clap(flatten, next_help_heading = "Prover")]
    pub prover_config: ProverConfig,
}

#[derive(Tabled, Serialize)]
pub struct BenchResult {
    #[tabled(rename = "Description")]
    pub description: String,
    #[tabled(rename = "Segments")]
    pub segments: u64,
    /// Total cycles run within guest
    #[tabled(rename = "Total Cycles")]
    pub total_cycles: u64,
    /// User cycles run within guest, slightly below total overhead cycles
    #[tabled(rename = "Cycles")]
    pub cycles: u64,
    #[tabled(rename = "Exec Time (secs)")]
    pub exec_time_secs: f64,
    #[tabled(rename = "Prove Time (secs)")]
    pub prove_time_secs: f64,
    #[tabled(rename = "Exec KHz")]
    pub exec_khz: f64,
    #[tabled(rename = "Prove KHz")]
    pub prove_khz: f64,
}

impl BenchArgs {
    pub async fn run(&self) -> Result<()> {
        let manifest = load_manifest(&self.common.manifest_path)?;

        self.prover_config
            .proving_backend
            .configure_proving_backend();

        // Currently only support Bento, not default prover
        let prover: BonsaiClient = BonsaiClient::from_env(risc0_zkvm::VERSION)?;

        let data_dir = self.common.data_dir.clone();
        let images_dir = data_dir.join("images");
        let inputs_dir = data_dir.join("inputs");

        let mut res = Vec::new();
        let mut count = 1;
        let total = manifest.entries.len();
        for entry in manifest.entries.iter() {
            tracing::info!("Running benchmark {count} of {total}...");
            let image_id = entry
                .image_id
                .clone()
                .expect("Image id missing from manifest");
            let input_id = entry
                .input_id
                .clone()
                .expect("Input id missing from manifest");

            let image_path = images_dir.join(format!("{}.elf", image_id));
            let input_path = inputs_dir.join(format!("{}.input", input_id));

            tracing::debug!("Loading image from {image_path:?}");
            let elf = std::fs::read(&image_path)
                .with_context(|| format!("Failed to load image file: {image_path:?}"))?;
            tracing::debug!("Loading image from {input_path:?}");
            let input = std::fs::read(&input_path)
                .with_context(|| format!("Failed to load input file: {input_path:?}"))?;

            tracing::debug!("Running program execution");
            let (session_stats, exec_elapsed_secs) = prove_bonsai(
                prover.clone(),
                image_id.clone(),
                elf.clone(),
                input.clone(),
                true,
            )
            .await
            .context("Execution failed")?;

            let prove_elapsed_secs = if self.exec_only {
                tracing::debug!("Exec only, skipping proof generation");
                0.0
            } else {
                tracing::debug!("Generating program proof");
                prove_bonsai(
                    prover.clone(),
                    image_id.clone(),
                    elf.clone(),
                    input.clone(),
                    self.exec_only,
                )
                .await
                .context("Proving failed")?
                .1
            };

            let exec_khz = if exec_elapsed_secs > 0.0 {
                session_stats.total_cycles as f64 / exec_elapsed_secs / 1000.0
            } else {
                0.0
            };
            let prove_khz = if prove_elapsed_secs > 0.0 {
                session_stats.total_cycles as f64 / prove_elapsed_secs / 1000.0
            } else {
                0.0
            };

            let bench_result = BenchResult {
                description: entry.description.clone(),
                segments: session_stats.segments as u64,
                total_cycles: session_stats.total_cycles,
                cycles: session_stats.cycles,
                exec_time_secs: exec_elapsed_secs,
                prove_time_secs: prove_elapsed_secs,
                exec_khz,
                prove_khz,
            };

            print_bench_result(&bench_result);

            res.push(bench_result);
            count += 1;
        }

        print_bench_summary(&res);

        if let Some(out_path) = self.json.clone() {
            let out_str = serde_json::to_string_pretty(&res)?;
            write(&out_path, &out_str).await?;
            tracing::info!("Wrote results to {:?}", out_path);
        }

        Ok(())
    }
}

fn print_bench_result(bench_result: &BenchResult) {
    let table_config = Settings::default()
        .with(Style::modern())
        .with(Reverse::columns(0))
        .with(Rotate::Left);
    println!("{}", Table::new(vec![bench_result]).with(table_config));
}

fn print_bench_summary(results: &[BenchResult]) {
    let avg_exec_mhz = results.iter().fold(0.0, |acc, x| acc + x.exec_khz) / results.len() as f64;
    let avg_prove_mhz = results.iter().fold(0.0, |acc, x| acc + x.prove_khz) / results.len() as f64;

    println!("Average Exec Mhz: {avg_exec_mhz}");
    println!("Average Prove Mhz: {avg_prove_mhz}");
}
