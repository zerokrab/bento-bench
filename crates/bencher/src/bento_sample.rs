use anyhow::{Context, Result, bail};
use bonsai_sdk::non_blocking::Client as ProvingClient;
use clap::Args;
use core::IterReq;
use risc0_zkvm::serde::to_vec;
use std::path::PathBuf;

use crate::stark_workflow;

#[derive(Args, Debug, Clone)]
pub struct BentoSampleArgs {
    /// Risc0 ZKVM elf file on disk
    #[clap(short = 'f', long)]
    elf_file: Option<PathBuf>,

    /// ZKVM encoded input to be supplied to ExecEnv .write() method
    ///
    /// Should be `risc0_zkvm::serde::to_vec` encoded binary data
    #[clap(short, long, conflicts_with = "iter_count")]
    input_file: Option<PathBuf>,

    /// Optional test vector to run the sample guest with the supplied iteration count
    ///
    /// Allows for rapid testing of arbitrary large cycle count guests
    ///
    /// NOTE: TODO remove this flag and simplify client
    #[clap(short = 'c', long, conflicts_with = "input_file")]
    iter_count: Option<u64>,

    /// Run a execute only job, aka preflight
    ///
    /// Useful for capturing metrics on a STARK proof like cycles.
    #[clap(short, long, default_value_t = false)]
    exec_only: bool,

    /// Bento HTTP API Endpoint
    #[clap(short = 't', long, default_value = "http://localhost:8081")]
    endpoint: String,

    /// Reserved worker count for job processing.
    ///
    /// Sets the reserved worker count for task scheduling. Jobs with higher
    /// reserved values get higher priority in cases where there are more than one task being
    /// progressed at a time.
    /// Default: 0
    #[clap(short, long, default_value_t = 0)]
    reserved: i32,
}

pub async fn run(args: BentoSampleArgs) -> Result<()> {
    // Format API key with reserved value if specified
    let api_key = if args.reserved != 0 {
        tracing::info!("Using reserved: {}", args.reserved);
        format!("v1:reserved:{}", args.reserved)
    } else {
        String::new()
    };

    let client = ProvingClient::from_parts(args.endpoint, api_key, risc0_zkvm::VERSION).unwrap();

    let (image, input) = if let Some(elf_file) = args.elf_file {
        let image = std::fs::read(elf_file).context("Failed to read elf file from disk")?;
        let input = std::fs::read(
            args.input_file
                .expect("if --elf-file is supplied, supply a --input-file"),
        )?;
        // let guest_env = GuestEnv::decode(&input);
        // let input = guest_env.stdin;
        (image, input)
    } else if let Some(iter_count) = args.iter_count {
        let input = to_vec(&IterReq::Iter(iter_count)).expect("Failed to r0 to_vec");
        let input = bytemuck::cast_slice(&input).to_vec();
        (guests::BENTO_SAMPLE_ELF.to_vec(), input)
    } else {
        bail!("Invalid arg config, either elf_file or iter_count should be supplied");
    };

    // Execute STARK workflow
    let (_session_uuid, _receipt_id) =
        stark_workflow(&client, image.clone(), input, vec![], args.exec_only).await?;

    // return if exec only and success
    if args.exec_only {
        return Ok(());
    }

    Ok(())
}
