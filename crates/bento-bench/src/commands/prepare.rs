use alloy::primitives::keccak256;
use anyhow::{Context, Result, bail};
use boundless_market::contracts::RequestInputType;
use boundless_market::storage::StorageDownloader;
use boundless_market::{GuestEnv, ProofRequest, StandardDownloader};
use risc0_zkvm::{ExecutorEnv, compute_image_id, default_executor};
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn fetch_image(url: &String, dir: &Path) -> Result<String> {
    let elf = StandardDownloader::new()
        .await
        .download_url(url.parse()?)
        .await?;
    let computed_image_id = compute_image_id(&elf)?.to_string();
    let image_id = computed_image_id.clone();
    let elf_path = dir.join(format!("{computed_image_id}.elf"));

    // Write the ELF to the data dir if it doesn't exist
    if !does_file_exist(&elf_path).await {
        std::fs::write(&elf_path, elf)
            .with_context(|| format!("Failed to write ELF file to {}", elf_path.display()))?;
    }

    Ok(image_id)
}

pub async fn fetch_input(request: &ProofRequest, out_dir: &Path) -> Result<String> {
    let input = match request.input.inputType {
        RequestInputType::Inline => GuestEnv::decode(&request.input.data)?.stdin,
        RequestInputType::Url => {
            let input_url =
                std::str::from_utf8(&request.input.data).context("Input URL is not valid UTF-8")?;
            tracing::debug!("Fetching input from {}", input_url);
            let input = StandardDownloader::new()
                .await
                .download_url(input_url.parse()?)
                .await?;
            GuestEnv::decode(&input)?.stdin
        }
        _ => bail!("Unsupported input type"),
    };

    let input_hash = save_input(input, out_dir)?;

    Ok(input_hash)
}

pub fn save_input(input: Vec<u8>, out_dir: &Path) -> Result<String> {
    let input_hash = keccak256(&input);

    let out_path = &out_dir.join(format!("{input_hash:x}.input"));
    std::fs::write(out_path, input)
        .with_context(|| format!("Failed to write input file to {}", out_path.display()))?;

    tracing::debug!("Saved input to {out_path:?}");

    Ok(format!("{input_hash:x}"))
}

pub async fn does_file_exist(path: &PathBuf) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

pub fn get_filename_without_extension(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(std::string::ToString::to_string)
}

pub async fn compute_cycles(input_in_path: &PathBuf, image_in_path: &PathBuf) -> Result<u64> {
    let input = fs::read(&input_in_path)
        .await
        .context("Failed to load input")?;

    let image = fs::read(&image_in_path)
        .await
        .context("Failed to load image")?;

    let env = ExecutorEnv::builder().write_slice(&input).build()?;
    let executor = default_executor();
    let session = executor.execute(env, &image).context("Execution failed")?;

    Ok(session.cycles())
}
