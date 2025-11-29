use crate::commands::datasheet::{config::DatasheetConfig, db::DbArgs};
use anyhow::Result;
use clap::{Args, Subcommand};

mod bench;
mod config;
mod db;
mod prepare;
use bench::BenchArgs;
use prepare::PrepareArgs;

#[derive(Args, Clone, Debug)]
pub struct DatasheetArgs {
    #[command(subcommand)]
    pub command: Datasheet,
    /// Prover configuration options
    #[clap(flatten, next_help_heading = "Datasheet Config")]
    pub config: DatasheetConfig,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Datasheet {
    Prepare(Box<PrepareArgs>),
    Bench(Box<BenchArgs>),
    Db(Box<DbArgs>),
}

impl DatasheetArgs {
    /// Run the datasheet command
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Datasheet::Bench(args) => {
                args.run(self.config.clone()).await?;
            }
            Datasheet::Prepare(args) => {
                args.run(self.config.clone()).await?;
            }
            Datasheet::Db(args) => {
                args.run(self.config.clone()).await?;
            }
        }
        Ok(())
    }
}
