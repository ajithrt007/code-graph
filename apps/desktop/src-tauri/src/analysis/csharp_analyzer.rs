//! C# / .NET analyzer built on top of Roslyn.
//!
//! The actual Roslyn work happens in the managed helper (`roslyn-sys`'s
//! `managed/RoslynBridge`). This module is the thin Rust orchestration
//! layer: it locates the target project/solution, hands the path to the
//! bridge, and converts the resulting JSON document into the
//! language-independent domain [`MethodGraph`].
//!
//! Keeping the analyzer small means Roslyn types never appear in the
//! application or domain layers — they live entirely inside
//! `roslyn-sys`.

use std::path::Path;

use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::domain::MethodGraph;

use super::solution_loader::SolutionLoader;

/// Analyzes C# / .NET solutions or projects.
#[derive(Debug, Default, Clone)]
pub struct CSharpAnalyzer {
    solution_loader: SolutionLoader,
}

impl CSharpAnalyzer {
    pub fn new() -> Self {
        Self {
            solution_loader: SolutionLoader::new(),
        }
    }

    /// Analyze a path (`.sln`, `.csproj`, or directory containing either).
    pub fn analyze_path(&self, path: &Path) -> Result<MethodGraph> {
        let projects = self
            .solution_loader
            .discover_projects(path)
            .with_context(|| format!("failed to discover projects at {}", path.display()))?;

        info!(projects = projects.len(), "discovered projects");
        let bridge = roslyn_sys::Bridge::init().context("initializing Roslyn bridge")?;

        let mut merged = MethodGraph::new();
        for project in projects {
            let json = bridge
                .analyze_to_json(&project.project_path)
                .with_context(|| {
                    format!(
                        "Roslyn bridge failed for project {}",
                        project.project_path.display()
                    )
                })?;
            debug!(project = %project.project_path.display(), "got graph JSON");
            let graph: MethodGraph = bridge.parse_graph(&json)?.into_domain()?.into();
            merge_into(&mut merged, graph);
        }
        Ok(merged)
    }
}

/// Merge `incoming` into `out`, deduplicating nodes by ID and edges by
/// (source, target).
fn merge_into(out: &mut MethodGraph, incoming: MethodGraph) {
    for (id, node) in incoming.methods {
        out.methods.entry(id).or_insert(node);
    }
    for edge in incoming.edges {
        out.add_edge(edge);
    }
}

// ---------------------------------------------------------------------------
// Boundary conversion: `roslyn-sys` mirrors the domain types so it can stay a
// self-contained crate; this `From` impl is the only place the two copies
// meet. Everything below is field-for-field and intentionally trivial.
// ---------------------------------------------------------------------------

impl From<roslyn_sys::domain::SourceLocation> for crate::domain::SourceLocation {
    fn from(v: roslyn_sys::domain::SourceLocation) -> Self {
        Self {
            file_path: v.file_path,
            start_line: v.start_line,
            start_column: v.start_column,
            end_line: v.end_line,
            end_column: v.end_column,
        }
    }
}

impl From<roslyn_sys::domain::MethodId> for crate::domain::MethodId {
    fn from(v: roslyn_sys::domain::MethodId) -> Self {
        Self(v.0)
    }
}

impl From<roslyn_sys::domain::MethodNode> for crate::domain::MethodNode {
    fn from(v: roslyn_sys::domain::MethodNode) -> Self {
        Self {
            id: v.id.into(),
            name: v.name,
            fully_qualified_name: v.fully_qualified_name,
            display_name: v.display_name,
            containing_type: v.containing_type,
            file_path: v.file_path,
            location: v.location.into(),
        }
    }
}

impl From<roslyn_sys::domain::RelationshipKind> for crate::domain::RelationshipKind {
    fn from(v: roslyn_sys::domain::RelationshipKind) -> Self {
        match v {
            roslyn_sys::domain::RelationshipKind::Calls => crate::domain::RelationshipKind::Calls,
        }
    }
}

impl From<roslyn_sys::domain::CallRelationship> for crate::domain::CallRelationship {
    fn from(v: roslyn_sys::domain::CallRelationship) -> Self {
        Self {
            source: v.source.into(),
            target: v.target.into(),
            kind: v.kind.into(),
        }
    }
}

impl From<roslyn_sys::domain::MethodGraph> for MethodGraph {
    fn from(v: roslyn_sys::domain::MethodGraph) -> Self {
        let mut out = MethodGraph::new();
        for (id, node) in v.methods {
            out.methods.insert(id.into(), node.into());
        }
        for edge in v.edges {
            out.add_edge(edge.into());
        }
        out
    }
}
