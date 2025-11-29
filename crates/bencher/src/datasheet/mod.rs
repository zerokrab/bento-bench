use alloy::primitives::U256;
use bonsai_sdk::responses::SessionStats;
use derive_more::Debug;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub mod db;

pub use db::GenerateRun;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub id: Option<Uuid>,
    pub notes: Option<String>,
    #[debug("{}",entries.len())]
    pub entries: Vec<ManifestEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub id: Option<Uuid>,
    /// Will be filled in if not provided by fetching the request
    pub image_id: Option<String>,
    /// Proof request id to fetch.
    pub request_id: U256,
    /// Description of the request
    pub description: String,
    /// Labeling data
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Datasheet {
    pub id: Option<Uuid>,
    pub manifest_id: Uuid,
    #[debug("{}",entries.len())]
    pub entries: Vec<DatasheetEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasheetEntry {
    pub id: Option<Uuid>,
    pub manifest_entry_id: Uuid,
    pub label: String,
    pub description: String,
    pub segments: u64,
    /// Total cycles run within guest
    pub total_cycles: u64,
    /// User cycles run within guest, slightly below total overhead cycles
    pub cycles: u64,
    pub exec_time_secs: f64,
    pub prove_time_secs: f64,
}
