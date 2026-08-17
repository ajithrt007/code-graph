//! Call-edge relationships between methods.

use serde::{Deserialize, Serialize};

use super::id::MethodId;

/// The kind of relationship between two methods. Currently we model direct
/// invocations; additional kinds (overrides, interface implementations,
/// constructors) can be added without breaking the model.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    /// The source method directly invokes the target method.
    Calls,
}

/// A directed relationship: `source` -> `target`.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallRelationship {
    pub source: MethodId,
    pub target: MethodId,
    pub kind: RelationshipKind,
}

/// Convenience alias used at the API surface.
pub type CallEdge = CallRelationship;
