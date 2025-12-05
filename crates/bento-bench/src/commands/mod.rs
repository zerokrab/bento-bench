use crate::commands::prepare_local::PrepareLocalArgs;
use crate::commands::prepare_request::PrepareRequestArgs;
use crate::commands::run_bench::RunArgs;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

mod prepare;
mod prepare_local;
mod prepare_request;
mod run_bench;

mod manifest;

#[derive(Parser, Debug)]
#[clap(about = "Bento benchmarking utility", arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Fetch and import a request from the market.
    PrepareRequest(Box<PrepareRequestArgs>),
    /// Import a local image and input.
    PrepareLocal(Box<PrepareLocalArgs>),
    /// Run a collection of benchmarks.
    Run(Box<RunArgs>),
}

#[derive(Args, Clone, Debug)]
pub struct CommonArgs {
    /// Directory to load images/inputs from
    #[clap(long, default_value = "./data")]
    data_dir: PathBuf,
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Command::PrepareRequest(args) => {
                args.run().await?;
            }
            Command::PrepareLocal(args) => {
                args.run().await?;
            }
            Command::Run(args) => {
                args.run().await?;
            }
        }
        Ok(())
    }
}
