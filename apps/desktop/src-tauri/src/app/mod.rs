//! Application / domain services.
//!
//! Sits between Tauri commands and the analysis layer. Owns the current
//! graph (in-memory), orchestrates analyses, and answers read queries.
//! Knows nothing about React Flow, Tauri, or any specific analyzer beyond
//! the [`crate::analysis::CSharpAnalyzer`] it currently uses.

pub mod errors;
pub mod graph_service;

pub use errors::AppError;
pub use graph_service::GraphService;
