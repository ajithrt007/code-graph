//! Tauri command implementations.
//!
//! Frontend-facing API. Keep these small — each delegates to the
//! application layer. Don't add business logic here.

use std::path::PathBuf;

use tauri::State;

use crate::app::graph_service::LoadedGraph;
use crate::domain::{MethodId, MethodNode};
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct AnalyzeArgs {
    pub path: String,
}

#[derive(serde::Deserialize)]
pub struct MethodArgs {
    pub id: String,
}

/// Analyze a .NET solution or project and replace the currently-loaded graph.
#[tauri::command]
pub fn analyze_solution(args: AnalyzeArgs, state: State<'_, AppState>) -> Result<LoadedGraph, crate::app::AppError> {
    state.graph.analyze(&PathBuf::from(args.path))
}

/// Return the currently-loaded graph.
#[tauri::command]
pub fn get_graph(state: State<'_, AppState>) -> Result<LoadedGraph, crate::app::AppError> {
    state.graph.current()
}

/// Return a single method by ID.
#[tauri::command]
pub fn get_method(args: MethodArgs, state: State<'_, AppState>) -> Result<MethodNode, crate::app::AppError> {
    state.graph.method(&MethodId::new(args.id))
}

/// Return all methods that directly call the method with the given ID.
#[tauri::command]
pub fn get_callers(args: MethodArgs, state: State<'_, AppState>) -> Result<Vec<MethodNode>, crate::app::AppError> {
    state.graph.callers(&MethodId::new(args.id))
}

/// Return all methods that the method with the given ID directly calls.
#[tauri::command]
pub fn get_callees(args: MethodArgs, state: State<'_, AppState>) -> Result<Vec<MethodNode>, crate::app::AppError> {
    state.graph.callees(&MethodId::new(args.id))
}