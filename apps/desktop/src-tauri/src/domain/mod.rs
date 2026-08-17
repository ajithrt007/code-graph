//! Language-independent graph/domain model.
//!
//! This module defines the data shapes that flow from analyzers into the
//! application layer and out to the frontend. It deliberately knows nothing
//! about Roslyn, React Flow, Tauri, or any storage backend. Analyzers for
//! additional languages should produce these same types.

pub mod call;
pub mod graph;
pub mod id;
pub mod method;

pub use call::{CallEdge, CallRelationship, RelationshipKind};
pub use graph::MethodGraph;
pub use id::MethodId;
pub use method::{MethodNode, SourceLocation};
