use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::call::CallRelationship;
use super::id::MethodId;
use super::method::MethodNode;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MethodGraph {
    pub methods: HashMap<MethodId, MethodNode>,
    pub edges: Vec<CallRelationship>,
}

impl MethodGraph {
    pub fn new() -> Self { Self::default() }
    pub fn add_method(&mut self, node: MethodNode) { self.methods.insert(node.id.clone(), node); }
    pub fn add_edge(&mut self, edge: CallRelationship) {
        if edge.source == edge.target { return; }
        if !self.methods.contains_key(&edge.source) || !self.methods.contains_key(&edge.target) { return; }
        if !self.edges.iter().any(|e| e.source == edge.source && e.target == edge.target) {
            self.edges.push(edge);
        }
    }
    pub fn callers_of(&self, target: &MethodId) -> Vec<&MethodNode> {
        self.edges.iter().filter(|e| &e.target == target).filter_map(|e| self.methods.get(&e.source)).collect()
    }
    pub fn callees_of(&self, source: &MethodId) -> Vec<&MethodNode> {
        self.edges.iter().filter(|e| &e.source == source).filter_map(|e| self.methods.get(&e.target)).collect()
    }
}
