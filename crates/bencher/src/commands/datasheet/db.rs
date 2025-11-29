use crate::commands::datasheet::config::DatasheetConfig;
use crate::datasheet::db::DatasheetDb;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

#[derive(Subcommand, Clone, Debug)]
pub enum RunsCmd {
    Latest,
    Get {
        uuid: uuid::Uuid,
    },
    All {
        #[clap(short, long, default_value_t = false)]
        verbose: bool,
    },
    Clear,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ManifestCmd {
    Latest,
    Get { uuid: uuid::Uuid },
    All,
    Info,
}
#[derive(Args, Clone, Debug)]
pub struct RunsArgs {
    #[command(subcommand)]
    pub command: RunsCmd,
}

#[derive(Args, Clone, Debug)]
pub struct ManifestArgs {
    #[command(subcommand)]
    pub command: ManifestCmd,
}

#[derive(Subcommand, Clone, Debug)]
pub enum DbCmd {
    Runs(RunsArgs),
    Manifest(ManifestArgs),
}

#[derive(Args, Clone, Debug)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCmd,
}

impl DbArgs {
    /// Run the datasheet generate command
    pub async fn run(&self, config: DatasheetConfig) -> Result<()> {
        let db_path = config.archive_dir.join("datasheet.db");
        let db = DatasheetDb::new(&db_path)
            .await
            .context("Failed to initialize SQLite database")?;
        match &self.command {
            DbCmd::Manifest(m) => match m.command {
                ManifestCmd::Latest => {
                    let latest_uuid = db.get_latest_manifest().await?;
                    let manifest = db.get_manifest_by_uuid(latest_uuid).await?;
                    println!("{}", serde_json::to_string_pretty(&manifest)?);
                }
                ManifestCmd::Get { uuid } => {
                    let manifest = db.get_manifest_by_uuid(uuid).await?;
                    println!("{}", serde_json::to_string_pretty(&manifest)?);
                }
                ManifestCmd::All => {
                    let manifests = db.get_all_manifests().await?;
                    for manifest in manifests {
                        println!("{}", manifest);
                    }
                }
                ManifestCmd::Info => {
                    let manifests = db.get_all_manifests().await?;
                    for id in manifests {
                        let manifest = db.get_manifest_by_uuid(id).await?;
                        println!("{:?}", manifest);
                    }
                }
            },
            DbCmd::Runs(r) => match r.command {
                RunsCmd::Latest => {
                    let latest_uuid = db.get_latest_datasheet().await?;
                    let datasheet = db.get_datasheet_by_uuid(latest_uuid).await?;
                    println!("{}", serde_json::to_string_pretty(&datasheet)?);
                }
                RunsCmd::Get { uuid } => {
                    let datasheet = db.get_datasheet_by_uuid(uuid).await?;
                    println!("{}", serde_json::to_string_pretty(&datasheet)?);
                }
                RunsCmd::All { verbose } => {
                    if !verbose {
                        let datasheets = db.get_all_datasheets().await?;
                        for datasheet in datasheets {
                            println!("{}", datasheet);
                        }
                        return Ok(());
                    }
                    let datasheets = db.get_all_datasheets_with_timestamps().await?;
                    for (uuid, timestamp) in datasheets {
                        println!("{} - {}", uuid, timestamp);
                    }
                }
                RunsCmd::Clear => {
                    db.clear_datasheets().await?;
                    println!("Cleared all datasheets from the database");
                }
            },
        }

        Ok(())
    }
}
