use alloy::primitives::keccak256;
use anyhow::{Context, Result, bail};
use boundless_market::contracts::RequestInputType;
use boundless_market::{GuestEnv, ProofRequest, storage::fetch_url};
use risc0_zkvm::{compute_image_id, default_executor, ExecutorEnv};
use std::path::PathBuf;
use tokio::fs;

pub async fn fetch_image(url: &String, dir: &PathBuf) -> Result<String> {
    let elf = fetch_url(&url).await?;
    let computed_image_id = compute_image_id(&elf)?.to_string();
    let image_id = computed_image_id.clone();
    tracing::info!("Computed Image ID: {}", computed_image_id);
    let elf_path = dir.join(format!("{}.elf", computed_image_id));

    // Write the ELF to the archive dir if it doesn't exist
    if !does_file_exist(&elf_path).await {
        tracing::debug!("Saved image to {elf_path:?}");
        std::fs::write(&elf_path, elf)
            .with_context(|| format!("Failed to write ELF file to {:?}", elf_path))?;
    }

    Ok(image_id)
}

pub async fn fetch_input(request: &ProofRequest, out_dir: &PathBuf) -> Result<String> {
    let input = match request.input.inputType {
        RequestInputType::Inline => GuestEnv::decode(&request.input.data)?.stdin,
        RequestInputType::Url => {
            let input_url =
                std::str::from_utf8(&request.input.data).context("Input URL is not valid UTF-8")?;
            tracing::debug!("Fetching input from {}", input_url);
            GuestEnv::decode(&fetch_url(input_url).await?)?.stdin
        }
        _ => bail!("Unsupported input type"),
    };

    let input_hash = save_input(input, out_dir)?;

    Ok(input_hash)
}

pub fn save_input(input: Vec<u8>, out_dir: &PathBuf) -> Result<String> {
    let input_hash = keccak256(&input);

    let out_path = &out_dir.join(format!("{:x}.input", input_hash));
    std::fs::write(&out_path, input)
        .with_context(|| format!("Failed to write input file to {:?}", out_path))?;

    tracing::debug!("Saved input to {out_path:?}");

    Ok(format!("{input_hash:x}"))
}

pub async fn does_file_exist(path: &PathBuf) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

pub fn get_filename_without_extension(path: &PathBuf) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

pub async fn compute_cycles(input_in_path: &PathBuf, image_in_path: &PathBuf) -> Result<u64> {
    let input = fs::read(&input_in_path).await.context("Failed to load input")?;

    let image = fs::read(&image_in_path)
        .await
        .context("Failed to load image")?;

    let env = ExecutorEnv::builder().write_slice(&input).build()?;
    let executor = default_executor();
    let session = executor.execute(env, &image).context("Execution failed")?;

    Ok(session.cycles())
}