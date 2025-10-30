use anyhow::Result;
use clap::{Parser, Subcommand};
#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Commands for running the Bento Sample Guest
    #[command(name = "bento-cli")]
    BentoGuest(bencher::bento_sample::BentoSampleArgs),
    Fetch(Box<bencher::commands::fetch::FetchAndSave>),
    Bench(Box<bencher::commands::bench::ProverBenchmark>),
}

#[derive(Parser, Debug)]
#[clap(about = "Bento Bencher CLI", arg_required_else_help = true)]
struct Args {
    /// Subcommand to run
    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    match args.command {
        Command::BentoGuest(bento_args) => {
            bencher::bento_sample::run(bento_args).await?;
        }
        Command::Fetch(fetch_args) => {
            fetch_args.run().await?;
        }
        Command::Bench(bencher) => {
            bencher.run().await?;
        }
    }
    Ok(())
}
