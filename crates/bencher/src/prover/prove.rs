use anyhow::{Context, Result, bail, ensure};
use bonsai_sdk::{non_blocking::Client as BonsaiClient, responses::SessionStats};
use hex::FromHex;
use risc0_zkvm::{Digest, compute_image_id};
use sqlx::{postgres::PgPool, postgres::PgPoolOptions};

use crate::DEFAULT_TASKDB_URL;

pub async fn prove_bonsai(
    prover: BonsaiClient,
    image_id: String,
    elf: Vec<u8>,
    input: Vec<u8>,
    exec_only: bool,
) -> Result<(SessionStats, f64)> {
    // Check if we can connect to PostgreSQL using environment variables
    let pg_pool = match create_pg_pool().await {
        Ok(pool) => {
            tracing::debug!("Connected to PostgreSQL database for enhanced metrics.");

            Some(pool)
        }
        Err(e) => {
            tracing::debug!(
                "Failed to connect to PostgreSQL database: {}, no enhanced metrics.",
                e
            );
            None
        }
    };
    ensure!(
        compute_image_id(&elf)? == Digest::from_hex(&image_id)?,
        "Image ID does not match ELF"
    );

    prover
        .upload_img(&image_id, elf)
        .await
        .context("Failed to upload ELF")?;
    tracing::debug!("Uploaded ELF to {}", &image_id);
    let input_id = prover
        .upload_input(input)
        .await
        .context("Failed to upload set-builder input")?;
    tracing::debug!("Uploaded input to {}", input_id);

    let assumptions = vec![];
    let start_time = std::time::Instant::now();

    let proof_id = prover
        .create_session(image_id, input_id, assumptions.clone(), exec_only)
        .await?;
    tracing::debug!("Created session {}", proof_id.uuid);

    let (stats, _elapsed_time) = loop {
        let status = proof_id.status(&prover).await?;

        match status.status.as_ref() {
            "RUNNING" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
            "SUCCEEDED" => {
                let Some(stats) = status.stats else {
                    bail!("Bento failed to return proof stats in response");
                };
                break (stats, status.elapsed_time);
            }
            _ => {
                let err_msg = status.error_msg.unwrap_or_default();
                bail!("stark proving failed: {err_msg}");
            }
        }
    };

    // Try to get effective KHz from PostgreSQL if available
    if let Some(ref pool) = pg_pool {
        let total_cycles_query = r#"
                    SELECT (output->>'total_cycles')::FLOAT8
                    FROM tasks
                    WHERE task_id = 'init' AND job_id = $1::uuid
                "#;

        let elapsed_secs_query = r#"
                    SELECT EXTRACT(EPOCH FROM (MAX(updated_at) - MIN(started_at)))::FLOAT8
                    FROM tasks
                    WHERE job_id = $1::uuid
                "#;

        let elapsed_result = sqlx::query_scalar::<_, f64>(elapsed_secs_query)
            .bind(proof_id.uuid.clone())
            .fetch_optional(pool)
            .await;

        match elapsed_result {
            Ok(Some(elapsed)) => {
                tracing::debug!("Retrieved from PostgreSQL: {} seconds", elapsed);
                Ok((stats, elapsed))
            }
            _ => {
                tracing::debug!(
                    "Failed to retrieve data from PostgreSQL, using client-side calculation"
                );
                let total_cycles: f64 = stats.total_cycles as f64;
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                Ok((stats, elapsed_secs))
            }
        }
    } else {
        tracing::debug!("No PostgreSQL data found for job, using client-side calculation.");
        let total_cycles: f64 = stats.total_cycles as f64;
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        Ok((stats, elapsed_secs))
    }
}
/// Create a PostgreSQL connection pool from environment variables
pub async fn create_pg_pool() -> Result<PgPool> {
    let database_url = if let Ok(url) = std::env::var("DATABASE_URL") {
        url
    } else {
        tracing::debug!(
            "DATABASE_URL not set; attempting to use DEFAULT_TASKDB_URL: {}",
            DEFAULT_TASKDB_URL
        );
        DEFAULT_TASKDB_URL.to_string()
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to PostgreSQL database")?;

    Ok(pool)
}
