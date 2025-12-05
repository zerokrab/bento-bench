use crate::commands::CommonArgs;
use crate::commands::manifest::{ManifestEntry, load_manifest, write_manifest};
use crate::commands::prepare::{compute_cycles, get_filename_without_extension, save_input};
use anyhow::{Context, Result, anyhow};
use clap::Args;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::fs;
use tokio::fs::create_dir_all;

#[derive(Args, Clone, Debug)]
pub struct PrepareLocalArgs {
    #[clap(flatten)]
    common: CommonArgs,
    /// Description of the image/input
    #[clap(long)]
    description: String,
    /// Path to image file
    #[clap(long)]
    image: PathBuf,
    /// Input string
    #[clap(long)]
    input: Option<String>,
    /// Path to input file
    #[clap(long)]
    input_path: Option<PathBuf>,
}

impl PrepareLocalArgs {
    pub async fn run(&self) -> Result<()> {
        let data_dir = self.common.data_dir.clone();

        let mut manifest = load_manifest(&self.common.data_dir)?;

        let images_dir = data_dir.join("images");
        create_dir_all(&images_dir).await.context(format!(
            "Failed to create images directory: {:?}",
            images_dir
        ))?;

        let inputs_dir = data_dir.join("inputs");
        create_dir_all(&inputs_dir).await.context(format!(
            "Failed to create inputs directory: {:?}",
            inputs_dir
        ))?;

        tracing::info!("Importing local data");

        let input_in_path = {
            if let Some(path) = self.input_path.clone() {
                Ok(path)
            } else if let Some(input_str) = self.input.clone() {
                let input_hash = save_input(input_str.into_bytes(), &inputs_dir)?; // TODO: Is this the correct encoding for a string?
                PathBuf::from_str(format!("{:?}/{}.input", &inputs_dir, &input_hash).as_str())
            } else {
                return Err(anyhow!("Must specify either --input or --input-path"));
            }
        }?;

        let image_in_path = self
            .input_path
            .clone()
            .ok_or(anyhow!("Image path required"))?;

        let input_out_path = inputs_dir.join(&input_in_path);
        let image_out_path = images_dir.join(&image_in_path);

        let input_id = get_filename_without_extension(&input_in_path)
            .ok_or(anyhow!("failed to parse input filename"))?;
        let image_id = get_filename_without_extension(&image_in_path)
            .ok_or(anyhow!("failed to parse image filename"))?;

        fs::copy(&input_in_path, input_out_path).await?;
        fs::copy(&image_in_path, image_out_path).await?;

        let cycles = compute_cycles(&input_in_path, &image_in_path).await?;

        let entry = ManifestEntry {
            description: self.description.clone(),
            request_id: None,
            input_id: Some(input_id),
            image_id: Some(image_id),
            cycles,
        };

        manifest.entries.push(entry);

        write_manifest(&manifest, &self.common.data_dir).await?;

        Ok(())
    }
}
