use crate::commands::refactor::bench::BenchArgs;
use crate::commands::refactor::prepare_local::PrepareLocalArgs;
use crate::commands::refactor::prepare_request::PrepareRequestArgs;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

mod bench;
mod prepare;
mod prepare_local;
mod prepare_request;

mod manifest;

#[derive(Args, Clone, Debug)]
pub struct RefactorArgs {
    #[command(subcommand)]
    pub command: Refactor,
}

#[derive(Args, Clone, Debug)]
pub struct CommonArgs {
    /// Path to manifest file
    #[clap(long = "manifest", default_value = "./manifest.json")]
    manifest_path: PathBuf,
    /// Directory to load images/inputs from
    #[clap(long, default_value = "./data")]
    data_dir: PathBuf,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Refactor {
    /// Fetch and import a request from the market
    PrepareRequest(Box<PrepareRequestArgs>),
    /// Import a local image and input
    PrepareLocal(Box<PrepareLocalArgs>),
    /// Run a collection of benchmarks
    Bench(Box<BenchArgs>),
}

impl RefactorArgs {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Refactor::PrepareRequest(args) => {
                args.run().await?;
            }
            Refactor::PrepareLocal(args) => {
                args.run().await?;
            }
            Refactor::Bench(args) => {
                args.run().await?;
            }
        }
        Ok(())
    }
}
