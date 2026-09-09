//! Thin project-scoped Tauri command surface.

use crate::app::graph_service::{LoadedGraph, MethodSource, ProjectSummary};
use crate::domain::{MethodId, MethodNode};
use crate::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[derive(serde::Deserialize)]
pub struct PathArgs {
    pub path: String,
}
#[derive(serde::Deserialize)]
pub struct ProjectArgs {
    pub project_id: String,
}
#[derive(serde::Deserialize)]
pub struct ProjectMethodArgs {
    pub project_id: String,
    pub id: String,
}
#[derive(serde::Deserialize)]
pub struct SaveMethodSourceArgs {
    pub project_id: String,
    pub id: String,
    pub code: String,
}

#[tauri::command]
pub fn list_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectSummary>, crate::app::AppError> {
    state.graph.list_projects()
}
#[tauri::command]
pub fn open_project(
    args: PathArgs,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<LoadedGraph, crate::app::AppError> {
    state.graph.open_project(&PathBuf::from(args.path), &app)
}
#[tauri::command]
pub fn load_project(
    args: ProjectArgs,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<LoadedGraph, crate::app::AppError> {
    state.graph.load_project(&args.project_id, &app)
}
#[tauri::command]
pub fn refresh_project(
    args: ProjectArgs,
    state: State<'_, AppState>,
) -> Result<LoadedGraph, crate::app::AppError> {
    state.graph.refresh_project(&args.project_id)
}
#[tauri::command]
pub fn close_project(args: ProjectArgs, state: State<'_, AppState>) {
    state.graph.close_project(&args.project_id)
}
#[tauri::command]
pub fn get_method(
    args: ProjectMethodArgs,
    state: State<'_, AppState>,
) -> Result<MethodNode, crate::app::AppError> {
    state
        .graph
        .method(&args.project_id, &MethodId::new(args.id))
}
#[tauri::command]
pub fn get_callers(
    args: ProjectMethodArgs,
    state: State<'_, AppState>,
) -> Result<Vec<MethodNode>, crate::app::AppError> {
    state
        .graph
        .callers(&args.project_id, &MethodId::new(args.id))
}
#[tauri::command]
pub fn get_callees(
    args: ProjectMethodArgs,
    state: State<'_, AppState>,
) -> Result<Vec<MethodNode>, crate::app::AppError> {
    state
        .graph
        .callees(&args.project_id, &MethodId::new(args.id))
}
#[tauri::command]
pub fn get_method_source(
    args: ProjectMethodArgs,
    state: State<'_, AppState>,
) -> Result<MethodSource, crate::app::AppError> {
    state
        .graph
        .method_source(&args.project_id, &MethodId::new(args.id))
}
#[tauri::command]
pub fn save_method_source(
    args: SaveMethodSourceArgs,
    state: State<'_, AppState>,
) -> Result<(), crate::app::AppError> {
    state
        .graph
        .save_method_source(&args.project_id, &MethodId::new(args.id), &args.code)
}
