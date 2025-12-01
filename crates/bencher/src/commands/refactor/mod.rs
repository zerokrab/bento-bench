use crate::commands::refactor::prepare_local::PrepareLocalArgs;
use crate::commands::refactor::prepare_request::PrepareRequestArgs;
use alloy::primitives::U256;
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

mod prepare;
mod prepare_local;
mod prepare_request;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub description: String,
    pub entries: Vec<ManifestEntryV2>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestEntryV2 {
    // TODO: Rename once other impl is removed
    /// Description of the request
    pub description: String,
    /// Proof request id to fetch.
    pub request_id: Option<U256>,
    pub input_id: Option<String>,
    pub image_id: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct RefactorArgs {
    #[command(subcommand)]
    pub command: Refactor,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Refactor {
    /// Fetch and import a request from the market
    PrepareRequest(Box<PrepareRequestArgs>),
    /// Import a local image and input
    PrepareLocal(Box<PrepareLocalArgs>),
    /// Run a collection of benchmarks
    Bench,
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
            // Refactor::Bench(args) => {
            //     args.run(self.config.clone()).await?;
            // }
            _ => {}
        }
        Ok(())
    }
}
