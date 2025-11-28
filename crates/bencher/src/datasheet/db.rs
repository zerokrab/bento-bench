use anyhow::{Context, Result};
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::types::time::OffsetDateTime;
use std::path::Path;
use uuid::Uuid;

use crate::datasheet::{Datasheet, Manifest, ManifestEntry};

use super::DatasheetEntry;

#[derive(Clone, Debug)]
pub struct GenerateRun {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub manifest_id: Option<i64>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub total_entries: i64,
    pub archive_dir: Option<String>,
    pub notes: Option<String>,
}

/// Database connection manager for datasheet entries
pub struct DatasheetDb {
    pool: SqlitePool,
}

impl DatasheetDb {
    /// Create a new database connection and run migrations
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Create parent directories if they don't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let db_url = format!("sqlite://{}", db_path.display());

        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&db_url)
            .await
            .unwrap_or(false)
        {
            sqlx::Sqlite::create_database(&db_url)
                .await
                .context("Failed to create database")?;
        }

        let connection_options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connection_options)
            .await
            .context("Failed to connect to SQLite database")?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;

        Ok(())
    }

    pub async fn insert_manifest(&self, manifest: &mut Manifest) -> Result<Uuid> {
        tracing::info!("Inserting manifest into database");
        if manifest.id.is_none() {
            manifest.id = Some(Uuid::new_v4());
        }
        let manifest_id = manifest.id.unwrap();
        // return error on conflict
        sqlx::query(
            r#"
            INSERT INTO manifests (uuid, notes)
            VALUES (?, ?)
            "#,
        )
        .bind(manifest.id)
        .bind(&manifest.notes)
        .execute(&self.pool)
        .await
        .context("Failed to insert manifest")?;
        let m_id = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM manifests WHERE uuid = ?
            "#,
        )
        .bind(manifest_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch manifest ID")?;

        for entry in manifest.entries.iter_mut() {
            let entry_id = self.insert_manifest_entry(entry).await?;

            self.link_manifest_entry(m_id, entry_id)
                .await
                .context("Failed to link manifest entry")?;
        }
        Ok(manifest_id)
    }

    /// Insert a manifest entry
    async fn insert_manifest_entry(&self, entry: &mut ManifestEntry) -> Result<i64> {
        tracing::info!("Inserting manifest entry into database");
        if entry.id.is_none() {
            entry.id = Some(Uuid::new_v4());
        }
        sqlx::query(
            r#"
            INSERT INTO manifest_entries (uuid, data) 
            VALUES (?, ?)
            ON CONFLICT (uuid) DO NOTHING
            "#,
        )
        .bind(entry.id.unwrap())
        .bind(sqlx::types::Json(&entry))
        .execute(&self.pool)
        .await
        .context("Failed to insert manifest entry")?;
        // the id of last inserted row
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM manifest_entries WHERE uuid = ?
            "#,
        )
        .bind(entry.id.unwrap())
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch manifest entry ID")?;

        Ok(id)
    }

    /// Link a manifest entry to a manifest
    async fn link_manifest_entry(&self, manifest_id: i64, entry_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO manifest_to_entry (manifest_id, entry_id)
            VALUES (?, ?)
            "#,
        )
        .bind(manifest_id)
        .bind(entry_id)
        .execute(&self.pool)
        .await
        .context("Failed to link manifest entry")?;

        Ok(())
    }

    pub async fn get_manifest_by_uuid(&self, id: Uuid) -> Result<Manifest> {
        let (row_id, uuid, notes) = sqlx::query_as::<_, (i64, Uuid, Option<String>)>(
            r#"
            SELECT id, uuid, notes
            FROM manifests WHERE uuid = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch manifest by UUID")?;

        let entries = sqlx::query_as::<_, (Uuid, sqlx::types::Json<ManifestEntry>)>(
            r#"
            SELECT me.uuid, me.data
            FROM manifest_entries me
            JOIN manifest_to_entry mte ON me.id = mte.entry_id
            WHERE mte.manifest_id = ?
            "#,
        )
        .bind(row_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch manifest entries")?;
        let manifest_entries: Vec<ManifestEntry> = entries
            .into_iter()
            .map(|(_uuid, json_entry)| json_entry.0)
            .collect();
        Ok(Manifest {
            id: Some(uuid),
            notes,
            entries: manifest_entries,
        })
    }

    pub async fn get_latest_manifest(&self) -> Result<Uuid> {
        let (_id, uuid) = sqlx::query_as::<_, (i64, Uuid)>(
            r#"
            SELECT id, uuid
            FROM manifests
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch latest manifest")?;

        Ok(uuid)
    }

    pub async fn get_all_manifests(&self) -> Result<Vec<Uuid>> {
        let rows = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT uuid
            FROM manifests
            ORDER BY id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch all manifests")?;

        Ok(rows.into_iter().map(|(uuid,)| uuid).collect())
    }

    pub async fn insert_datasheet(&self, datasheet: &mut Datasheet) -> Result<Uuid> {
        tracing::debug!("Inserting datasheet into database");
        if datasheet.id.is_none() {
            datasheet.id = Some(Uuid::new_v4());
        }
        let datasheet_id = datasheet.id.unwrap();
        // return error on conflict
        sqlx::query(
            r#"
            INSERT INTO datasheets (uuid, manifest_id)
            VALUES (?, ?)
            "#,
        )
        .bind(datasheet.id)
        .bind(datasheet.manifest_id) // manifest_id can be set later
        .execute(&self.pool)
        .await
        .context("Failed to insert datasheet")?;

        let d_id = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM datasheets WHERE uuid = ?
            "#,
        )
        .bind(datasheet_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch datasheet ID")?;

        for entry in datasheet.entries.iter_mut() {
            let entry_id = self.insert_datasheet_entry(entry).await?;

            self.link_datasheet_entry(d_id, entry_id)
                .await
                .context("Failed to link datasheet entry")?;
        }
        Ok(datasheet_id)
    }

    async fn insert_datasheet_entry(&self, entry: &mut DatasheetEntry) -> Result<i64> {
        tracing::debug!("Inserting datasheet entry into database");
        if entry.id.is_none() {
            entry.id = Some(Uuid::new_v4());
        }
        sqlx::query(
            r#"
            INSERT INTO datasheet_entries (uuid, data, manifest_entry_id) 
            VALUES (?, ?, ?)
            "#,
        )
        .bind(entry.id.unwrap())
        .bind(sqlx::types::Json(&entry))
        .bind(entry.manifest_entry_id)
        .execute(&self.pool)
        .await
        .context("Failed to insert datasheet entry")?;
        // the id of last inserted row
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM datasheet_entries WHERE uuid = ?
            "#,
        )
        .bind(entry.id.unwrap())
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch datasheet entry ID")?;
        Ok(id)
    }

    async fn link_datasheet_entry(&self, datasheet_id: i64, entry_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO datasheet_to_entry (datasheet_id, entry_id)
            VALUES (?, ?)
            "#,
        )
        .bind(datasheet_id)
        .bind(entry_id)
        .execute(&self.pool)
        .await
        .context("Failed to link datasheet entry")?;
        Ok(())
    }

    pub async fn get_datasheet_by_uuid(&self, id: Uuid) -> Result<Datasheet> {
        let (row_id, uuid, manifest_id) = sqlx::query_as::<_, (i64, Uuid, Uuid)>(
            r#"
            SELECT id, uuid, manifest_id
            FROM datasheets WHERE uuid = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch datasheet by UUID")?;

        let entries = sqlx::query_as::<_, (Uuid, sqlx::types::Json<DatasheetEntry>)>(
            r#"
            SELECT de.uuid, de.data
            FROM datasheet_entries de
            JOIN datasheet_to_entry dte ON de.id = dte.entry_id
            WHERE dte.datasheet_id = ?
            "#,
        )
        .bind(row_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch datasheet entries")?;
        let datasheet_entries: Vec<DatasheetEntry> = entries
            .into_iter()
            .map(|(_uuid, json_entry)| json_entry.0)
            .collect();
        Ok(Datasheet {
            id: Some(uuid),
            manifest_id,
            entries: datasheet_entries,
        })
    }

    pub async fn get_all_datasheets(&self) -> Result<Vec<Uuid>> {
        let rows = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT uuid
            FROM datasheets
            ORDER BY id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch all datasheets")?;

        Ok(rows.into_iter().map(|(uuid,)| uuid).collect())
    }

    pub async fn get_latest_datasheet(&self) -> Result<Uuid> {
        let (_id, uuid) = sqlx::query_as::<_, (i64, Uuid)>(
            r#"
            SELECT id, uuid
            FROM datasheets
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch latest datasheet")?;

        Ok(uuid)
    }

    pub async fn get_all_datasheets_with_timestamps(&self) -> Result<Vec<(Uuid, OffsetDateTime)>> {
        let rows = sqlx::query_as::<_, (Uuid, OffsetDateTime)>(
            r#"
            SELECT uuid, timestamp
            FROM datasheets
            ORDER BY timestamp DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch all datasheets with timestamps")?;

        Ok(rows)
    }

    pub async fn clear_datasheets(&self) -> Result<()> {
        tracing::info!("Clearing all datasheets from the database");
        sqlx::query(
            r#"
            DELETE FROM datasheet_to_entry;
            DELETE FROM datasheet_entries;
            DELETE FROM datasheets;
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to clear datasheets from database")?;
        Ok(())
    }
}
