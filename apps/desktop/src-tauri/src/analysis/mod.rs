//! Language analyzers.
//!
//! Each analyzer consumes a project/solution of some language and
//! produces a [`crate::domain::MethodGraph`]. Analyzers must not leak
//! language-specific types past this module; everything upstream must
//! be expressed in the language-independent domain types.
//!
//! Adding a new language is a matter of implementing a new analyzer and
//! registering it in [`crate::app::graph_service::GraphService`].

pub mod csharp_analyzer;
pub mod solution_loader;

pub use csharp_analyzer::CSharpAnalyzer;
pub use solution_loader::SolutionLoader;
