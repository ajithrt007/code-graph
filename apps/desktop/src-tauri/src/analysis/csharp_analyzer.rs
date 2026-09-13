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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::domain::MethodGraph;

use super::solution_loader::SolutionLoader;

/// One progress update emitted while a project loads. Forwarded by the
/// application layer as `project-load-progress` events; `current`/`total`
/// count analyzed projects (`0/0` = an indeterminate step such as
/// discovery).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LoadProgress {
    pub target: String,
    pub stage: String,
    pub detail: String,
    pub current: usize,
    pub total: usize,
}

impl LoadProgress {
    fn new(target: &str, stage: &str, detail: String, current: usize, total: usize) -> Self {
        Self {
            target: target.to_string(),
            stage: stage.to_string(),
            detail,
            current,
            total,
        }
    }
}

/// Analyzes C# / .NET solutions or projects.
///
/// `resource_dir` is the installed app's Tauri resource directory, where the
/// self-contained RoslynBridge bundle ships inside the installer (see
/// `bundle.resources` in `tauri.conf.json`). When absent (dev/test), the
/// bridge falls back to the publish directory in the source tree.
#[derive(Debug, Default, Clone)]
pub struct CSharpAnalyzer {
    solution_loader: SolutionLoader,
    resource_dir: Option<PathBuf>,
}

impl CSharpAnalyzer {
    pub fn new() -> Self {
        Self {
            solution_loader: SolutionLoader::new(),
            resource_dir: None,
        }
    }

    pub fn with_resource_dir(dir: PathBuf) -> Self {
        Self {
            solution_loader: SolutionLoader::new(),
            resource_dir: Some(dir),
        }
    }

    /// Analyze a path (`.sln`, `.csproj`, or directory containing either).
    ///
    /// `progress` receives a [`LoadProgress`] per stage (discovery, each
    /// project's analysis, merge); the caller forwards these to the UI.
    /// Analysis itself stays synchronous — events are best-effort.
    pub fn analyze_path(&self, path: &Path, progress: &dyn Fn(LoadProgress)) -> Result<MethodGraph> {
        let target = path.to_string_lossy().to_string();
        let emit = |stage: &str, detail: String, current: usize, total: usize| {
            progress(LoadProgress::new(&target, stage, detail, current, total));
        };

        emit(
            "discover",
            format!("Looking for .sln or .csproj in {}", path.display()),
            0,
            0,
        );
        let projects = self
            .solution_loader
            .discover_projects(path)
            .with_context(|| format!("failed to discover projects at {}", path.display()))?;

        let total = projects.len();
        let names = projects
            .iter()
            .map(|p| {
                p.project_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| p.project_path.display().to_string())
            })
            .collect::<Vec<_>>()
            .join(", ");
        emit(
            "discover",
            format!("Found {total} project(s): {names}"),
            0,
            total,
        );

        info!(projects = projects.len(), "discovered projects");
        let bridge = match &self.resource_dir {
            Some(dir) => roslyn_sys::Bridge::init_with_resource_dir(dir),
            None => roslyn_sys::Bridge::init(),
        }
        .context("initializing Roslyn bridge")?;
        emit(
            "init",
            format!(
                "Analyzer ready: {} (self-contained, no SDK needed)",
                bridge.helper_path().display()
            ),
            0,
            total,
        );

        let mut merged = MethodGraph::new();
        for (index, project) in projects.iter().enumerate() {
            let name = project
                .project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&names);
            emit(
                "analyze",
                format!(
                    "Analyzing {}/{}: {} — running RoslynBridge…",
                    index + 1,
                    total,
                    name
                ),
                index,
                total,
            );
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
            emit(
                "analyze",
                format!(
                    "Parsed {}/{}: {} — {} methods, {} calls",
                    index + 1,
                    total,
                    name,
                    graph.methods.len(),
                    graph.edges.len()
                ),
                index + 1,
                total,
            );
            merge_into(&mut merged, graph);
        }
        emit(
            "done",
            format!(
                "Done — {} methods, {} calls across {total} project(s)",
                merged.methods.len(),
                merged.edges.len()
            ),
            total,
            total,
        );
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
