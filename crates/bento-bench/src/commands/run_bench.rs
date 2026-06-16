use crate::ProverConfig;
use crate::commands::CommonArgs;
use crate::commands::fetch::fetch_suite;
use crate::commands::manifest::load_manifest;
use crate::prove::{prove_snark, prove_stark};
use anyhow::{Context, Result, anyhow};
use bonsai_sdk::non_blocking::Client as BonsaiClient;
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;
use tabled::derive::display;
use tabled::settings::{Reverse, Rotate, Settings, Style};
use tabled::{Table, Tabled};
use tokio::fs::write;

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[clap(flatten)]
    common: CommonArgs,

    /// Fetch a suite from a URL (.tar.zst) instead of using --data-dir
    #[clap(long)]
    fetch: Option<String>,

    /// Execute only (no prove)
    #[clap(long, default_value_t = false)]
    exec_only: bool,

    /// Additionally create a snark proof for each benchmark
    #[clap(long = "snark", default_value_t = false)]
    run_snark: bool,

    /// Use Taskdb to measure proof duration instead of client-side timing
    #[clap(long = "check-taskdb", default_value_t = false)]
    check_taskdb: bool,

    /// Output summary to json file
    #[clap(long)]
    json: Option<PathBuf>,

    /// Polling interval when checking job status (ms). When `check-taskdb=false` this may impact timing precision
    #[clap(long = "poll-interval", default_value_t = 1000)]
    poll_interval_ms: u64,

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
    #[tabled(rename = "Exec Duration (secs)")]
    pub exec_secs: f64,
    #[tabled(
        rename = "Prove Duration (secs)",
        display("display::option", "Skipped")
    )]
    pub stark_secs: Option<f64>,
    #[tabled(
        rename = "Snark Duration (secs)",
        display("display::option", "Skipped")
    )]
    pub snark_secs: Option<f64>,
    #[tabled(rename = "Total Duration (secs)")]
    pub total_secs: f64,
    #[tabled(rename = "Exec MHz")]
    pub exec_mhz: f64,
    #[tabled(rename = "Prove MHz", display("display::option", "Skipped"))]
    pub prove_mhz: Option<f64>,
}

#[derive(Tabled, Serialize, Debug, Clone)]
pub struct BenchSummary {
    #[tabled(rename = "Exec Min MHz")]
    pub exec_min_mhz: f64,
    #[tabled(rename = "Exec Max MHz")]
    pub exec_max_mhz: f64,
    #[tabled(rename = "Exec Avg MHz")]
    pub exec_avg_mhz: f64,
    #[tabled(rename = "Exec Median MHz")]
    pub exec_median_mhz: f64,
    #[tabled(rename = "Prove Min MHz", display("display::option", "Skipped"))]
    pub prove_min_mhz: Option<f64>,
    #[tabled(rename = "Prove Max MHz", display("display::option", "Skipped"))]
    pub prove_max_mhz: Option<f64>,
    #[tabled(rename = "Prove Avg MHz", display("display::option", "Skipped"))]
    pub prove_avg_mhz: Option<f64>,
    #[tabled(rename = "Prove Median MHz", display("display::option", "Skipped"))]
    pub prove_median_mhz: Option<f64>,
}

#[derive(Serialize)]
pub struct BenchJsonOutput {
    runs: Vec<BenchResult>,
    summary: BenchSummary,
}

impl RunArgs {
    pub async fn run(&self) -> Result<()> {
        let data_dir = if let Some(ref url) = self.fetch {
            fetch_suite(url).await?
        } else {
            self.common.data_dir.clone()
        };

        let manifest = load_manifest(&data_dir, false)?;

        self.prover_config
            .proving_backend
            .configure_proving_backend();

        // Currently only support Bento, not default prover
        let prover: BonsaiClient = BonsaiClient::from_env(risc0_zkvm::VERSION)?;

        let images_dir = data_dir.join("images");
        let inputs_dir = data_dir.join("inputs");

        let mut res = Vec::new();
        let mut count = 1;
        let total = manifest.entries.len();
        for entry in manifest.entries {
            tracing::info!(
                "Running benchmark {count} / {total} ({0} cycles) - {1}...",
                entry.cycles,
                entry.description
            );
            let image_id = entry
                .image_id
                .clone()
                .expect("Image id missing from manifest");
            let input_id = entry
                .input_id
                .clone()
                .expect("Input id missing from manifest");

            let image_path = images_dir.join(format!("{image_id}.elf"));
            let input_path = inputs_dir.join(format!("{input_id}.input"));

            tracing::debug!("Loading image from {image_path:?}");
            let elf = std::fs::read(&image_path)
                .with_context(|| format!("Failed to load image file: {}", image_path.display()))?;
            tracing::debug!("Loading image from {input_path:?}");
            let input = std::fs::read(&input_path)
                .with_context(|| format!("Failed to load input file: {}", input_path.display()))?;

            tracing::debug!("Running program preflight...");
            let (_, session_stats, exec_duration_secs, exec_mhz) = prove_stark(
                prover.clone(),
                image_id.clone(),
                elf.clone(),
                input.clone(),
                true,
                self.check_taskdb,
                self.poll_interval_ms,
            )
            .await
            .context("Execution failed")?;

            tracing::debug!("Running stark proof...");
            let (session_id, stark_duration_secs, stark_mhz) = if self.exec_only {
                tracing::debug!("Exec only, skipping proof generation");
                (None, None, None)
            } else {
                tracing::debug!("Generating program proof");
                let (session_id, _, stark_duration, stark_mhz) = prove_stark(
                    prover.clone(),
                    image_id.clone(),
                    elf.clone(),
                    input.clone(),
                    self.exec_only,
                    self.check_taskdb,
                    self.poll_interval_ms,
                )
                .await
                .context("Stark proving failed")?;

                (Some(session_id), Some(stark_duration), Some(stark_mhz))
            };

            let snark_duration_secs = if self.run_snark {
                tracing::debug!("Running snark proof...");
                let session_id = session_id.ok_or_else(|| anyhow!("Missing stark session id"))?;
                let (_, snark_duration) = prove_snark(
                    prover.clone(),
                    session_id,
                    self.check_taskdb,
                    self.poll_interval_ms,
                )
                .await?;
                Some(snark_duration)
            } else {
                tracing::debug!("Skipping snark proof");
                None
            };

            let bench_result = BenchResult {
                description: entry.description.clone(),
                segments: session_stats.segments as u64,
                total_cycles: session_stats.total_cycles,
                cycles: session_stats.cycles,
                exec_secs: exec_duration_secs,
                stark_secs: stark_duration_secs,
                snark_secs: snark_duration_secs,
                total_secs: exec_duration_secs
                    + stark_duration_secs.unwrap_or(0.0)
                    + snark_duration_secs.unwrap_or(0.0),
                exec_mhz,
                prove_mhz: stark_mhz,
            };

            print_bench_result(&bench_result);

            res.push(bench_result);
            count += 1;
        }

        let summary = get_bench_summary(&res);

        print_bench_summary(&summary);

        if let Some(json_path) = self.json.clone() {
            save_json(&json_path, res, &summary).await?;
            tracing::info!("Wrote summary to {:?}", json_path);
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

fn print_bench_summary(bench_summary: &BenchSummary) {
    let table_config = Settings::default()
        .with(Style::modern())
        .with(Reverse::columns(0))
        .with(Rotate::Left);

    println!("{}", Table::new(vec![bench_summary]).with(table_config));
}

fn get_bench_summary(results: &[BenchResult]) -> BenchSummary {
    let mut exec_res: Vec<f64> = results.iter().map(|r| r.exec_mhz).collect();
    let min_exec_mhz = exec_res.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_exec_mhz = exec_res.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let avg_exec_mhz = exec_res.iter().fold(0.0, |acc, x| acc + x) / results.len() as f64;
    let median_exec_mhz = median(&mut exec_res).unwrap_or(0.0);

    if !results.is_empty() && results[0].prove_mhz.is_some() {
        let mut prove_res: Vec<f64> = results.iter().map(|r| r.prove_mhz.unwrap()).collect();
        let min_prove_mhz = prove_res.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_prove_mhz = prove_res.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg_prove_mhz = prove_res.iter().fold(0.0, |acc, x| acc + x) / results.len() as f64;
        let median_prove_mhz = median(&mut prove_res).unwrap_or(0.0);

        BenchSummary {
            exec_min_mhz: min_exec_mhz,
            exec_max_mhz: max_exec_mhz,
            exec_avg_mhz: avg_exec_mhz,
            exec_median_mhz: median_exec_mhz,
            prove_min_mhz: Some(min_prove_mhz),
            prove_max_mhz: Some(max_prove_mhz),
            prove_avg_mhz: Some(avg_prove_mhz),
            prove_median_mhz: Some(median_prove_mhz),
        }
    } else {
        BenchSummary {
            exec_min_mhz: min_exec_mhz,
            exec_max_mhz: max_exec_mhz,
            exec_avg_mhz: avg_exec_mhz,
            exec_median_mhz: median_exec_mhz,
            prove_min_mhz: None,
            prove_max_mhz: None,
            prove_avg_mhz: None,
            prove_median_mhz: None,
        }
    }
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let len = values.len();
    let mid = len / 2;

    if len.is_multiple_of(2) {
        // Even number of elements: average the two middle values
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        // Odd number of elements: return the middle value
        Some(values[mid])
    }
}

async fn save_json(
    out_path: &PathBuf,
    runs: Vec<BenchResult>,
    summary: &BenchSummary,
) -> Result<()> {
    let out = BenchJsonOutput {
        runs,
        summary: summary.clone(),
    };
    let out_str = serde_json::to_string_pretty(&out)?;
    write(&out_path, &out_str).await?;

    Ok(())
}
