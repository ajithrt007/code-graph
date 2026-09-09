//! Application-layer error type.
//!
//! Implements `serde::Serialize` so it can be returned from Tauri commands
//! without leaking internal types to the frontend. The frontend receives a
//! flat shape with a `kind` discriminator.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no graph has been loaded yet")]
    NoGraphLoaded,
    #[error("method not found: {0}")]
    MethodNotFound(String),
    #[error("analysis failed: {0}")]
    Analysis(String),
    #[error("persistence failed: {0}")]
    Persistence(String),
    #[error("project service has not initialized")]
    NotInitialized,
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("project is not open: {0}")]
    ProjectNotOpen(String),
    #[error("project is unavailable: {0}")]
    ProjectUnavailable(String),
    #[error("file watcher failed: {0}")]
    Watch(String),
    #[error("could not read method source: {0}")]
    Source(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[serde(tag = "kind", content = "message", rename_all = "snake_case")]
        enum Kind<'a> {
            NoGraphLoaded,
            MethodNotFound(&'a str),
            Analysis(&'a str),
            Persistence(&'a str),
            NotInitialized,
            ProjectNotFound(&'a str),
            ProjectNotOpen(&'a str),
            ProjectUnavailable(&'a str),
            Watch(&'a str),
            Source(&'a str),
        }
        let value = match self {
            AppError::NoGraphLoaded => Kind::NoGraphLoaded,
            AppError::MethodNotFound(id) => Kind::MethodNotFound(id),
            AppError::Analysis(msg) => Kind::Analysis(msg),
            AppError::Persistence(msg) => Kind::Persistence(msg),
            AppError::NotInitialized => Kind::NotInitialized,
            AppError::ProjectNotFound(id) => Kind::ProjectNotFound(id),
            AppError::ProjectNotOpen(id) => Kind::ProjectNotOpen(id),
            AppError::ProjectUnavailable(path) => Kind::ProjectUnavailable(path),
            AppError::Watch(msg) => Kind::Watch(msg),
            AppError::Source(msg) => Kind::Source(msg),
        };
        value.serialize(serializer)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Analysis(err.to_string())
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
