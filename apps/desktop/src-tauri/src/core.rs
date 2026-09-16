use crate::permissions::Grant;
use serde::Serialize;
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(String),
    #[error("Approval required: {0}")]
    Permission(String),
    #[error("Local file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Project database could not be accessed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Invalid project data: {0}")]
    Json(#[from] serde_json::Error),
}
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
pub type Result<T> = std::result::Result<T, AppError>;
pub struct AppState {
    pub root: PathBuf,
    pub pool: SqlitePool,
    pub grants: Mutex<HashMap<String, Grant>>,
    pub tasks: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub writes: Mutex<()>,
    pub _log_guard: tracing_appender::non_blocking::WorkerGuard,
}
