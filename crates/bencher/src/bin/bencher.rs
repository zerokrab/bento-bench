use anyhow::Result;
use clap::{Parser, Subcommand};
#[derive(Subcommand, Clone, Debug)]
enum Command {
    Datasheet(Box<bencher::commands::datasheet::DatasheetArgs>),
    Refactor(Box<bencher::commands::refactor::RefactorArgs>),
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
        Command::Datasheet(datasheet_args) => {
            datasheet_args.run().await?;
        }
        Command::Refactor(refactor_args) => {
            refactor_args.run().await?;
        }
    }
    Ok(())
}
