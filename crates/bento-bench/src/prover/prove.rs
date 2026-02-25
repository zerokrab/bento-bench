use anyhow::{Context, Result, anyhow, bail, ensure};
use bonsai_sdk::non_blocking::{SessionId, SnarkId};
use bonsai_sdk::{non_blocking::Client as BonsaiClient, responses::SessionStats};
use hex::FromHex;
use risc0_zkvm::{Digest, compute_image_id};
use sqlx::{postgres::PgPool, postgres::PgPoolOptions};

use crate::DEFAULT_TASKDB_URL;

pub async fn prove_stark(
    prover: BonsaiClient,
    image_id: String,
    elf: Vec<u8>,
    input: Vec<u8>,
    exec_only: bool,
    check_taskdb: bool,
) -> Result<(SessionId, SessionStats, f64, f64)> {
    // Optional postgres connection to get taskdb stats
    let pg_pool = if check_taskdb {
        create_pg_pool().await
    } else {
        None
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

    let session_id = prover
        .create_session(image_id, input_id, assumptions.clone(), exec_only)
        .await?;
    tracing::debug!("Created session {}", session_id.uuid);

    let (stats, _elapsed_time) = loop {
        let status = session_id.status(&prover).await?;

        match status.status.as_ref() {
            "RUNNING" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                continue;
            }
            "SUCCEEDED" => {
                let Some(stats) = status.stats else {
                    bail!("Bento failed to return stark proof stats in response");
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
    let elapsed_secs = if let Some(ref pool) = pg_pool {
        get_taskdb_duration_secs(pool, &session_id.uuid.to_string()).await?
    } else {
        tracing::debug!("No PostgreSQL data found for job, using client-side calculation.");
        start_time.elapsed().as_secs_f64()
    };

    let khz = if elapsed_secs > 0.0 {
        stats.total_cycles as f64 / elapsed_secs / 1000.0
    } else {
        0.0
    };

    Ok((session_id, stats, elapsed_secs, khz))
}

pub async fn prove_snark(
    prover: BonsaiClient,
    session_id: SessionId,
    check_taskdb: bool,
) -> Result<(SnarkId, f64)> {
    let pool = if check_taskdb {
        create_pg_pool().await
    } else {
        None
    };
    let start_time = std::time::Instant::now();

    let snark_id = prover.create_snark(session_id.uuid.clone()).await?;

    loop {
        let status = snark_id.status(&prover).await?;

        match status.status.as_ref() {
            "RUNNING" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
            "SUCCEEDED" => {
                break;
            }
            _ => {
                let err_msg = status.error_msg.unwrap_or_default();
                bail!("snark proving failed: {err_msg}");
            }
        }
    }

    // Try to get effective KHz from PostgreSQL if available
    if let Some(ref pool) = pool {
        let elapsed_secs = get_taskdb_duration_secs(pool, &snark_id.uuid.to_string()).await?;
        Ok((snark_id, elapsed_secs))
    } else {
        tracing::debug!("No PostgreSQL data found for job, using client-side calculation.");
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        Ok((snark_id, elapsed_secs))
    }
}

/// Create a PostgreSQL connection pool from environment variables
pub async fn create_pg_pool() -> Option<PgPool> {
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
        .context("Failed to connect to PostgreSQL database");

    match pool {
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
    }
}

/// Query taskdb for the tasks matching `job_id` to compute start/end time => job duration.
pub async fn get_taskdb_duration_secs(pool: &PgPool, job_id: &str) -> Result<f64> {
    let elapsed_secs_query = r#"
                SELECT EXTRACT(EPOCH FROM (MAX(updated_at) - MIN(started_at)))::FLOAT8
                FROM tasks
                WHERE job_id = $1::uuid
            "#;

    let elapsed_result = sqlx::query_scalar::<_, f64>(elapsed_secs_query)
        .bind(job_id)
        .fetch_optional(pool)
        .await;

    if let Ok(Some(elapsed)) = elapsed_result {
        tracing::debug!("Retrieved from taskdb: {} seconds", elapsed);
        Ok(elapsed)
    } else {
        tracing::debug!(
            "Failed to retrieve data from taskdb, reverting to client-side calculation"
        );
        Err(anyhow!("Failed to get duration from taskdb"))
    }
}
