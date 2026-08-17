//! `roslyn-sys`: thin Rust facade over Microsoft.CodeAnalysis.
//!
//! Implementation strategy
//! -----------------------
//! `Microsoft.CodeAnalysis` is a managed (.NET) library. This crate wraps
//! it behind a small process boundary:
//!
//! 1. A managed helper assembly (`managed/RoslynBridge`) does the Roslyn
//!    work and prints a JSON graph document to stdout.
//! 2. [`Bridge`] picks the self-contained bundle for the current OS/arch and
//!    spawns its native executable directly, reading the JSON back. No
//!    `dotnet` runtime/SDK is needed at runtime.
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
