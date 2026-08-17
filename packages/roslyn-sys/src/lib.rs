//! `roslyn-sys`: thin Rust facade over Microsoft.CodeAnalysis.
//!
//! Implementation strategy
//! -----------------------
//! `Microsoft.CodeAnalysis` is a managed (.NET) library. This crate wraps
//! it behind a small process boundary:
//!
//! 1. A managed helper assembly (`managed/RoslynBridge`) does the Roslyn
//!    work and prints a JSON graph document to stdout.
//! 2. [`Bridge`] spawns `dotnet RoslynBridge.dll <path>` on demand and
//!    reads the JSON back.
//! 3. The `dto` module converts that wire format into the crate's
//!    [`domain`] value types, which the desktop analyzer consumes.
//!
//! Only JSON strings cross the boundary, so Roslyn types never appear past
//! this crate's `dto` layer.

pub mod analysis;
pub mod bridge;
pub mod domain;
pub mod dto;

pub use bridge::Bridge;
pub use dto::AnalysisGraph;
