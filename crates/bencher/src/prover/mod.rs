use anyhow::{Context, Result, bail};
use bonsai_sdk::non_blocking::Client as ProvingClient;
use risc0_zkvm::{Receipt, compute_image_id};

pub async fn stark_workflow(
    client: &ProvingClient,
    image: Vec<u8>,
    input: Vec<u8>,
    assumptions: Vec<String>,
    exec_only: bool,
) -> Result<(String, String)> {
    // elf/image
    let image_id = compute_image_id(&image).unwrap();
    let image_id_str = image_id.to_string();
    client
        .upload_img(&image_id_str, image)
        .await
        .context("Failed to upload image")?;

    // input
    let input_id = client
        .upload_input(input)
        .await
        .context("Failed to upload input")?;

    tracing::info!("image_id: {image_id} | input_id: {input_id}");

    let session = client
        .create_session(image_id_str.clone(), input_id, assumptions, exec_only)
        .await
        .context("STARK proof failure")?;
    tracing::info!("STARK job_id: {}", session.uuid);

    let mut receipt_id = String::new();

    loop {
        let res = session
            .status(client)
            .await
            .context("Failed to get STARK status")?;

        match res.status.as_ref() {
            "RUNNING" => {
                tracing::info!("STARK Job running....");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
            "SUCCEEDED" => {
                tracing::info!("Job done!");
                if exec_only {
                    break;
                }
                let receipt_bytes = client
                    .receipt_download(&session)
                    .await
                    .context("Failed to download receipt")?;

                let receipt: Receipt = bincode::deserialize(&receipt_bytes).unwrap();
                receipt.verify(image_id).unwrap();

                receipt_id = client
                    .upload_receipt(receipt_bytes.clone())
                    .await
                    .context("Failed to upload receipt")?;

                break;
            }
            _ => {
                bail!(
                    "Job failed: {} - {}",
                    session.uuid,
                    res.error_msg.as_ref().unwrap_or(&String::new())
                );
            }
        }
    }
    Ok((session.uuid, receipt_id))
}
