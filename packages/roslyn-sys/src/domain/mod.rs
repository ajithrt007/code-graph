//! Language-independent graph/domain types used inside `roslyn-sys`.
//!
//! These are *value types* for moving data around; they intentionally
//! mirror the desktop crate's domain types so a single `From` impl at the
//! boundary converts between them. Keeping the copies separate avoids
//! `roslyn-sys` depending on the desktop crate.

pub mod call;
pub mod graph;
pub mod id;
pub mod method;

pub use call::{CallEdge, CallRelationship, RelationshipKind};
pub use graph::MethodGraph;
pub use id::MethodId;
pub use method::{MethodNode, SourceLocation};
