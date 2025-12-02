pub mod commands;
pub mod prover;
pub use prover::*;
pub const DEFAULT_BENTO_API_URL: &str = "http://localhost:8081";
pub const DEFAULT_TASKDB_URL: &str = "postgresql://worker:password@localhost:5432/taskdb";
