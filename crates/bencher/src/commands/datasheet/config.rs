use std::path::PathBuf;

use clap::Args;

use crate::ProverConfig;

/// Configuration options for commands that utilize proving.
#[derive(Args, Debug, Clone)]
pub struct DatasheetConfig {
    #[clap(long, short = 'd', default_value = "data")]
    pub archive_dir: PathBuf,
    #[clap(long, short = 'm')]
    pub manifest_path: Option<PathBuf>,
    #[clap(flatten, next_help_heading = "Prover")]
    pub prover_config: ProverConfig,
}
