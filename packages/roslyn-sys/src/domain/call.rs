use serde::{Deserialize, Serialize};

use super::id::MethodId;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    Calls,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallRelationship {
    pub source: MethodId,
    pub target: MethodId,
    pub kind: RelationshipKind,
}

pub type CallEdge = CallRelationship;
