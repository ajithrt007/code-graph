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
        }
        let value = match self {
            AppError::NoGraphLoaded => Kind::NoGraphLoaded,
            AppError::MethodNotFound(id) => Kind::MethodNotFound(id),
            AppError::Analysis(msg) => Kind::Analysis(msg),
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
