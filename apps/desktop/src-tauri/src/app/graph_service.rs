//! Read/write services over the current graph.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::analysis::CSharpAnalyzer;
use crate::domain::{MethodGraph, MethodId, MethodNode};

use super::errors::{AppError, AppResult};

/// What the frontend sees as the "current analysis": a graph plus a
/// pointer to where it came from. We deliberately keep it small and flat
/// so serialization is predictable.
#[derive(Debug, Clone, Serialize)]
pub struct LoadedGraph {
    pub source_path: PathBuf,
    pub graph: MethodGraph,
}

/// Owns the currently-loaded graph. Thread-safe via a [`Mutex`]; reads are
/// cheap clones of `Arc<MethodGraph>` in larger systems, but a plain lock
/// is fine for the single-writer, occasional-reader pattern here.
#[derive(Debug, Default)]
pub struct GraphService {
    inner: Mutex<Option<LoadedGraph>>,
}

impl GraphService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run analysis on `path` and replace the current graph.
    pub fn analyze(&self, path: &Path) -> AppResult<LoadedGraph> {
        let analyzer = CSharpAnalyzer::new();
        let graph = analyzer
            .analyze_path(path)
            .map_err(|e| AppError::Analysis(e.to_string()))?;
        let loaded = LoadedGraph {
            source_path: path.to_path_buf(),
            graph,
        };
        *self.inner.lock().expect("graph service poisoned") = Some(loaded.clone());
        Ok(loaded)
    }

    pub fn current(&self) -> AppResult<LoadedGraph> {
        self.inner
            .lock()
            .expect("graph service poisoned")
            .clone()
            .ok_or(AppError::NoGraphLoaded)
    }

    pub fn method(&self, id: &MethodId) -> AppResult<MethodNode> {
        let loaded = self.current()?;
        loaded
            .graph
            .methods
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::MethodNotFound(id.to_string()))
    }

    pub fn callers(&self, id: &MethodId) -> AppResult<Vec<MethodNode>> {
        let loaded = self.current()?;
        let mut out: Vec<MethodNode> = loaded
            .graph
            .callers_of(id)
            .into_iter()
            .cloned()
            .collect();
        out.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(out)
    }

    pub fn callees(&self, id: &MethodId) -> AppResult<Vec<MethodNode>> {
        let loaded = self.current()?;
        let mut out: Vec<MethodNode> = loaded
            .graph
            .callees_of(id)
            .into_iter()
            .cloned()
            .collect();
        out.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(out)
    }
}
