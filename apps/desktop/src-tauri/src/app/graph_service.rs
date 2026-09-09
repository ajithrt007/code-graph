//! Project-scoped graph state, persistence, and filesystem watching.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::analysis::CSharpAnalyzer;
use crate::domain::{MethodGraph, MethodId, MethodNode};

use super::errors::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub path: PathBuf,
    pub display_name: String,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedGraph {
    pub project_id: String,
    pub source_path: PathBuf,
    pub graph: MethodGraph,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodSource {
    pub code: String,
    pub start_line: u32,
}

pub struct GraphService {
    database: Mutex<Option<Connection>>,
    graphs: Mutex<HashMap<String, LoadedGraph>>,
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl Default for GraphService {
    fn default() -> Self {
        Self {
            database: Mutex::new(None),
            graphs: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
        }
    }
}

impl GraphService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&self, app: &AppHandle) -> AppResult<()> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::Persistence(e.to_string()))?;
        fs::create_dir_all(&data_dir).map_err(|e| AppError::Persistence(e.to_string()))?;
        let connection = Connection::open(data_dir.join("codegraph.sqlite3"))
            .map_err(|e| AppError::Persistence(e.to_string()))?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                last_opened_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL
             );",
            )
            .map_err(|e| AppError::Persistence(e.to_string()))?;
        *self.database.lock().expect("database poisoned") = Some(connection);
        Ok(())
    }

    pub fn list_projects(&self) -> AppResult<Vec<ProjectSummary>> {
        let database = self.database.lock().expect("database poisoned");
        let connection = database.as_ref().ok_or(AppError::NotInitialized)?;
        let mut statement = connection.prepare(
            "SELECT id, path, display_name, last_opened_at FROM projects ORDER BY last_opened_at DESC, display_name ASC",
        ).map_err(|e| AppError::Persistence(e.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    display_name: row.get(2)?,
                    last_opened_at: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Persistence(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Persistence(e.to_string()))
    }

    pub fn open_project(&self, path: &Path, app: &AppHandle) -> AppResult<LoadedGraph> {
        let canonical = path
            .canonicalize()
            .map_err(|e| AppError::Analysis(format!("cannot open {}: {e}", path.display())))?;
        let project = self.upsert_project(&canonical)?;
        let loaded = self.analyze_project(&project.id, &canonical)?;
        self.install_watcher(&project, app)?;
        Ok(loaded)
    }

    pub fn load_project(&self, project_id: &str, app: &AppHandle) -> AppResult<LoadedGraph> {
        if let Some(loaded) = self
            .graphs
            .lock()
            .expect("graphs poisoned")
            .get(project_id)
            .cloned()
        {
            return Ok(loaded);
        }
        let project = self.project_by_id(project_id)?;
        if !project.path.exists() {
            return Err(AppError::ProjectUnavailable(
                project.path.display().to_string(),
            ));
        }
        let loaded = self.analyze_project(&project.id, &project.path)?;
        self.install_watcher(&project, app)?;
        Ok(loaded)
    }

    pub fn refresh_project(&self, project_id: &str) -> AppResult<LoadedGraph> {
        let project = self.project_by_id(project_id)?;
        self.analyze_project(&project.id, &project.path)
    }

    pub fn close_project(&self, project_id: &str) {
        self.graphs
            .lock()
            .expect("graphs poisoned")
            .remove(project_id);
        self.watchers
            .lock()
            .expect("watchers poisoned")
            .remove(project_id);
    }

    pub fn method(&self, project_id: &str, id: &MethodId) -> AppResult<MethodNode> {
        self.current(project_id)?
            .graph
            .methods
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::MethodNotFound(id.to_string()))
    }

    pub fn callers(&self, project_id: &str, id: &MethodId) -> AppResult<Vec<MethodNode>> {
        let loaded = self.current(project_id)?;
        let mut methods: Vec<_> = loaded.graph.callers_of(id).into_iter().cloned().collect();
        methods.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(methods)
    }

    pub fn callees(&self, project_id: &str, id: &MethodId) -> AppResult<Vec<MethodNode>> {
        let loaded = self.current(project_id)?;
        let mut methods: Vec<_> = loaded.graph.callees_of(id).into_iter().cloned().collect();
        methods.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(methods)
    }

    pub fn method_source(&self, project_id: &str, id: &MethodId) -> AppResult<MethodSource> {
        let method = self.method(project_id, id)?;
        let source =
            fs::read_to_string(&method.file_path).map_err(|e| AppError::Source(e.to_string()))?;
        let lines: Vec<&str> = source.lines().collect();
        let start = method.location.start_line.saturating_sub(1) as usize;
        let end = method.location.end_line as usize;
        Ok(MethodSource {
            code: lines.get(start..end).unwrap_or_default().join("\n"),
            start_line: method.location.start_line,
        })
    }

    pub fn save_method_source(&self, project_id: &str, id: &MethodId, code: &str) -> AppResult<()> {
        let method = self.method(project_id, id)?;
        let source =
            fs::read_to_string(&method.file_path).map_err(|e| AppError::Source(e.to_string()))?;
        let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();
        let start = method.location.start_line.saturating_sub(1) as usize;
        let end = method.location.end_line as usize;
        if start >= lines.len() || end > lines.len() || start >= end {
            return Err(AppError::Source(
                "method source range is no longer valid; refresh the graph first".into(),
            ));
        }
        let replacement: Vec<String> = code.lines().map(str::to_owned).collect();
        lines.splice(start..end, replacement);
        let trailing_newline = source.ends_with('\n');
        let mut next = lines.join("\n");
        if trailing_newline {
            next.push('\n');
        }
        fs::write(&method.file_path, next).map_err(|e| AppError::Source(e.to_string()))
    }

    fn current(&self, project_id: &str) -> AppResult<LoadedGraph> {
        self.graphs
            .lock()
            .expect("graphs poisoned")
            .get(project_id)
            .cloned()
            .ok_or_else(|| AppError::ProjectNotOpen(project_id.to_owned()))
    }

    fn analyze_project(&self, project_id: &str, path: &Path) -> AppResult<LoadedGraph> {
        let graph = CSharpAnalyzer::new()
            .analyze_path(path)
            .map_err(|e| AppError::Analysis(e.to_string()))?;
        let loaded = LoadedGraph {
            project_id: project_id.to_owned(),
            source_path: path.to_path_buf(),
            graph,
        };
        self.graphs
            .lock()
            .expect("graphs poisoned")
            .insert(project_id.to_owned(), loaded.clone());
        Ok(loaded)
    }

    fn upsert_project(&self, path: &Path) -> AppResult<ProjectSummary> {
        let now = now_unix();
        let path_string = path.to_string_lossy().to_string();
        let display_name = path
            .file_stem()
            .or_else(|| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled project")
            .to_owned();
        let database = self.database.lock().expect("database poisoned");
        let connection = database.as_ref().ok_or(AppError::NotInitialized)?;
        let existing: Option<String> = connection
            .query_row(
                "SELECT id FROM projects WHERE path = ?1",
                [&path_string],
                |row| row.get(0),
            )
            .ok();
        let id =
            existing.unwrap_or_else(|| format!("project-{now}-{}", stable_path_hash(&path_string)));
        connection.execute(
            "INSERT INTO projects (id, path, display_name, last_opened_at, created_at) VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(path) DO UPDATE SET display_name = excluded.display_name, last_opened_at = excluded.last_opened_at",
            params![id, path_string, display_name, now],
        ).map_err(|e| AppError::Persistence(e.to_string()))?;
        Ok(ProjectSummary {
            id,
            path: path.to_path_buf(),
            display_name,
            last_opened_at: now,
        })
    }

    fn project_by_id(&self, project_id: &str) -> AppResult<ProjectSummary> {
        let database = self.database.lock().expect("database poisoned");
        let connection = database.as_ref().ok_or(AppError::NotInitialized)?;
        connection
            .query_row(
                "SELECT id, path, display_name, last_opened_at FROM projects WHERE id = ?1",
                [project_id],
                |row| {
                    Ok(ProjectSummary {
                        id: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        display_name: row.get(2)?,
                        last_opened_at: row.get(3)?,
                    })
                },
            )
            .map_err(|_| AppError::ProjectNotFound(project_id.to_owned()))
    }

    fn install_watcher(&self, project: &ProjectSummary, app: &AppHandle) -> AppResult<()> {
        if self
            .watchers
            .lock()
            .expect("watchers poisoned")
            .contains_key(&project.id)
        {
            return Ok(());
        }
        let project_id = project.id.clone();
        let handle = app.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                if let Ok(event) = result {
                    if is_relevant_change(&event) {
                        let _ = handle.emit("project-files-changed", &project_id);
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| AppError::Watch(e.to_string()))?;
        let root = if project.path.is_dir() {
            project.path.clone()
        } else {
            project.path.parent().unwrap_or(&project.path).to_path_buf()
        };
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| AppError::Watch(e.to_string()))?;
        self.watchers
            .lock()
            .expect("watchers poisoned")
            .insert(project.id.clone(), watcher);
        Ok(())
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn stable_path_hash(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}
fn is_relevant_change(event: &Event) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && event.paths.iter().any(|path| {
            !path
                .components()
                .any(|part| matches!(part.as_os_str().to_str(), Some(".git" | "bin" | "obj")))
                && matches!(
                    path.extension().and_then(|v| v.to_str()),
                    Some("cs" | "csproj" | "sln" | "props" | "targets")
                )
        })
}
