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

/// One matching line inside a method's source span, for global search.
#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    pub method: MethodNode,
    pub line_number: u32,
    pub line_text: String,
}

/// Caps for global search: at most this many lines per method and this
/// many matches overall, so a common token can't flood the palette.
const MAX_SEARCH_LINES_PER_METHOD: usize = 5;
const MAX_SEARCH_RESULTS: usize = 100;

pub struct GraphService {
    database: Mutex<Option<Connection>>,
    graphs: Mutex<HashMap<String, LoadedGraph>>,
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
    /// Installed app's Tauri resource directory, resolved once at startup.
    /// The RoslynBridge bundle ships inside the installer there; `None`
    /// (e.g. resource dir unavailable) just means the bridge falls back to
    /// the dev-tree publish directory / env override.
    resource_dir: Mutex<Option<PathBuf>>,
}

impl Default for GraphService {
    fn default() -> Self {
        Self {
            database: Mutex::new(None),
            graphs: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
            resource_dir: Mutex::new(None),
        }
    }
}

impl GraphService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&self, app: &AppHandle) -> AppResult<()> {
        *self.resource_dir.lock().expect("resource dir poisoned") = app.path().resource_dir().ok();
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

    /// Case-insensitive substring search over every method's source span.
    /// Returns one item per matching line (method + whole line), ordered by
    /// method display name then line number. Methods whose files can't be
    /// read are skipped rather than failing the whole search.
    pub fn search_methods(&self, project_id: &str, query: &str) -> AppResult<Vec<SearchMatch>> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let loaded = self.current(project_id)?;
        let mut methods: Vec<&MethodNode> = loaded.graph.methods.values().collect();
        methods.sort_by(|a, b| a.display_name.cmp(&b.display_name));

        let mut out = Vec::new();
        for method in methods {
            if out.len() >= MAX_SEARCH_RESULTS {
                break;
            }
            let Ok(source) = fs::read_to_string(&method.file_path) else {
                continue;
            };
            let lines: Vec<&str> = source.lines().collect();
            for (line_number, line_text) in find_line_matches(
                &lines,
                method.location.start_line,
                method.location.end_line,
                &needle,
                MAX_SEARCH_LINES_PER_METHOD,
            ) {
                out.push(SearchMatch {
                    method: method.clone(),
                    line_number,
                    line_text,
                });
                if out.len() >= MAX_SEARCH_RESULTS {
                    break;
                }
            }
        }
        Ok(out)
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
        let analyzer = match self
            .resource_dir
            .lock()
            .expect("resource dir poisoned")
            .clone()
        {
            Some(dir) => CSharpAnalyzer::with_resource_dir(dir),
            None => CSharpAnalyzer::new(),
        };
        let graph = analyzer
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

/// Case-insensitive substring matches within a method's 1-based line span.
/// Returns `(line_number, trimmed_text)` ordered by line, capped at `max`.
/// `needle` must already be lowercased by the caller.
fn find_line_matches(
    all_lines: &[&str],
    start_line: u32,
    end_line: u32,
    needle: &str,
    max: usize,
) -> Vec<(u32, String)> {
    let start = start_line.saturating_sub(1) as usize;
    let end = (end_line as usize).min(all_lines.len());
    if start >= end || start >= all_lines.len() || max == 0 {
        return Vec::new();
    }
    all_lines[start..end]
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains(needle))
        .take(max)
        .map(|(idx, line)| ((start + idx + 1) as u32, line.trim().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::find_line_matches;
    use super::{GraphService, LoadedGraph};
    use crate::domain::{MethodGraph, MethodId, MethodNode, SourceLocation};
    use std::collections::HashMap;
    use std::path::PathBuf;

    const LINES: &[&str] = &[
        "public Order? GetOrder(int id)",
        "{",
        "    return _repository.FindById(id);",
        "}",
    ];

    #[test]
    fn matches_declaration_and_body_case_insensitively() {
        let out = find_line_matches(LINES, 1, 4, "findbyid", 10);
        assert_eq!(
            out,
            vec![(3, "return _repository.FindById(id);".to_string())]
        );

        let out = find_line_matches(LINES, 1, 4, "getorder", 10);
        assert_eq!(out, vec![(1, "public Order? GetOrder(int id)".to_string())]);
    }

    #[test]
    fn respects_span_and_cap() {
        // Body line excluded when the span covers only the declaration.
        assert!(find_line_matches(LINES, 1, 1, "findbyid", 10).is_empty());
        // Cap of 1 keeps only the first of two matches for "{"-adjacent "r".
        let out = find_line_matches(LINES, 1, 4, "r", 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 1);
    }

    #[test]
    fn rejects_degenerate_spans() {
        assert!(find_line_matches(LINES, 3, 2, "x", 10).is_empty());
        assert!(find_line_matches(LINES, 99, 120, "x", 10).is_empty());
        assert!(find_line_matches(LINES, 1, 4, "getorder", 0).is_empty());
    }

    fn test_service(source_file: &std::path::Path) -> GraphService {
        let node = |id: &str, display: &str, start: u32, end: u32| MethodNode {
            id: MethodId::new(id),
            name: display.to_string(),
            fully_qualified_name: display.to_string(),
            display_name: display.to_string(),
            containing_type: "Svc".to_string(),
            file_path: source_file.to_string_lossy().to_string(),
            location: SourceLocation {
                file_path: source_file.to_string_lossy().to_string(),
                start_line: start,
                start_column: 1,
                end_line: end,
                end_column: 1,
            },
        };
        let mut methods = HashMap::new();
        methods.insert(MethodId::new("b"), node("b", "Svc.B()", 5, 7));
        methods.insert(MethodId::new("a"), node("a", "Svc.A()", 1, 3));
        let service = GraphService::new();
        service.graphs.lock().expect("graphs poisoned").insert(
            "p".to_string(),
            LoadedGraph {
                project_id: "p".to_string(),
                source_path: PathBuf::from("/tmp"),
                graph: MethodGraph {
                    methods,
                    edges: vec![],
                },
            },
        );
        service
    }

    #[test]
    fn search_methods_matches_lines_ordered_by_method() {
        let dir = std::env::temp_dir().join(format!(
            "codegraph-search-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let file = dir.join("Svc.cs");
        std::fs::write(
            &file,
            [
                "void A() {",
                "  Log(\"hit\");",
                "}",
                "",
                "void B() {",
                "  Log(\"HIT\");",
                "}",
            ]
            .join("\n"),
        )
        .expect("write temp source");

        let service = test_service(&file);
        let out = service.search_methods("p", "hit").expect("search");
        assert_eq!(out.len(), 2);
        // Ordered by display name: A() before B().
        assert_eq!(out[0].method.display_name, "Svc.A()");
        assert_eq!(out[0].line_number, 2);
        assert_eq!(out[0].line_text, "Log(\"hit\");");
        assert_eq!(out[1].method.display_name, "Svc.B()");
        assert_eq!(out[1].line_number, 6);

        assert!(service
            .search_methods("p", "   ")
            .expect("search")
            .is_empty());
        assert!(service.search_methods("missing", "hit").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
