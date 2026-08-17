//! Tauri command surface.
//!
//! Every command is a thin wrapper around [`crate::app::GraphService`]. The
//! command shape (argument + return type) is the contract with the
//! frontend.

pub mod commands;

pub use commands::*;
