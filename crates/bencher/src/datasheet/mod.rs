use alloy::primitives::U256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Manifest(
    /// Names of programs to benchmark
    pub HashMap<String, Vec<ManifestEntry>>,
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ManifestEntry {
    /// Will be filled in if not provided by fetching the request
    pub image_id: Option<String>,
    /// Proof request id to fetch.
    pub request_id: U256,
    /// Description of the request
    pub description: String,
    /// Optional UUID for the entry
    pub uuid: Option<uuid::Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasheetEntry {
    pub label: String,
    pub uuid: uuid::Uuid,
    pub description: String,
    pub num_cycles: f64,
    pub elapsed_time_secs: f64,
}
