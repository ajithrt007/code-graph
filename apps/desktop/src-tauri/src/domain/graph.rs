//! Aggregate graph type returned by analyzers.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::call::CallRelationship;
use super::id::MethodId;
use super::method::MethodNode;

/// A method-call graph for a codebase.
///
/// Methods are stored in a map keyed by stable `MethodId` so callers can
/// efficiently look up a node by ID. Edges are stored as a flat vector; the
/// domain layer does not precompute adjacency lists because the graph is
/// typically small enough that ad-hoc filtering is fast and keeps the data
/// structure easy to serialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MethodGraph {
    pub methods: HashMap<MethodId, MethodNode>,
    pub edges: Vec<CallRelationship>,
}

impl MethodGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_method(&mut self, node: MethodNode) {
        self.methods.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: CallRelationship) {
        // Drop self-loops and dangling edges so downstream layers don't have
        // to defend against them.
        if edge.source == edge.target {
            return;
        }
        if !self.methods.contains_key(&edge.source)
            || !self.methods.contains_key(&edge.target)
        {
            return;
        }
        // Deduplicate.
        if !self.edges.iter().any(|e| e.source == edge.source && e.target == edge.target) {
            self.edges.push(edge);
        }
    }

    /// Return all methods that directly call `target`.
    pub fn callers_of(&self, target: &MethodId) -> Vec<&MethodNode> {
        self.edges
            .iter()
            .filter(|e| &e.target == target)
            .filter_map(|e| self.methods.get(&e.source))
            .collect()
    }

    /// Return all methods that `source` directly calls.
    pub fn callees_of(&self, source: &MethodId) -> Vec<&MethodNode> {
        self.edges
            .iter()
            .filter(|e| &e.source == source)
            .filter_map(|e| self.methods.get(&e.target))
            .collect()
    }
}
