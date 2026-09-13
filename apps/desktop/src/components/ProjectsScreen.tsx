import { useEffect, useRef, useState, type FormEvent } from "react";
import type { ProjectSummary } from "../domain/method";
import type { OpenLogLine } from "../workspace/useWorkspace";
import { UpdateSection } from "./UpdateSection";

export interface OpenProgress {
  current: number;
  total: number;
  stage: string;
}

export function ProjectsScreen({
  projects,
  opening,
  log,
  progress,
  failed,
  onOpenPath,
  onOpenRecent,
  onDeleteProject,
  onDismissLog,
}: {
  projects: ProjectSummary[];
  opening: boolean;
  log: OpenLogLine[];
  progress: OpenProgress | null;
  failed: boolean;
  onOpenPath: (path: string) => void;
  onOpenRecent: (id: string) => void;
  onDeleteProject: (id: string) => void;
  onDismissLog: () => void;
}) {
  const [path, setPath] = useState("");
  // Two-click delete: first click arms the button ("Confirm?"), second
  // click deletes. No native confirm dialog involved.
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  useEffect(() => {
    if (!pendingDelete) return;
    const timer = window.setTimeout(() => setPendingDelete(null), 4000);
    return () => window.clearTimeout(timer);
  }, [pendingDelete]);
  const logRef = useRef<HTMLDivElement>(null);
  const showTerminal = opening || failed;

  useEffect(() => {
    const el = logRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [log, opening]);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (path.trim() && !opening) onOpenPath(path.trim());
  };

  const copyLog = () => {
    const text = log.map((line) => `[${line.time}] ${line.text}`).join("\n");
    void navigator.clipboard.writeText(text).catch(() => undefined);
  };

  const percent =
    progress && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : null;

  return (
    <section className="projects-screen">
      <form className="project-add" onSubmit={submit}>
        <h1>Open a project</h1>
        <p>Enter a .sln, .csproj, or project-folder path.</p>
        <input
          autoFocus
          value={path}
          disabled={opening}
          onChange={(event) => setPath(event.target.value)}
          placeholder="/path/to/YourSolution.sln"
        />
        <button disabled={!path.trim() || opening}>
          {opening ? "Opening…" : "Open project"}
        </button>
        <UpdateSection />
        {showTerminal && (
          <div className="load-terminal" role="status" aria-label="Project loading log">
            <div className="load-terminal__header">
              <span>{failed ? "Loading failed" : "Loading project…"}</span>
              <span className="load-terminal__actions">
                <button type="button" onClick={copyLog} disabled={log.length === 0}>
                  Copy log
                </button>
                {failed && (
                  <button type="button" onClick={onDismissLog}>
                    Dismiss
                  </button>
                )}
              </span>
            </div>
            {percent !== null && (
              <div className="load-terminal__bar" aria-label={`Loading progress ${percent}%`}>
                <div
                  className="load-terminal__fill"
                  style={{ width: `${percent}%` }}
                />
              </div>
            )}
            <div className="load-terminal__meta">
              {percent !== null
                ? `${percent}% — ${progress?.current}/${progress?.total} projects`
                : "Discovering projects…"}
            </div>
            <div ref={logRef} className="load-terminal__lines">
              {log.length === 0 ? (
                <div className="load-terminal__line">Starting…</div>
              ) : (
                log.map((line, index) => (
                  <div key={index} className="load-terminal__line">
                    <span className="load-terminal__time">[{line.time}]</span>{" "}
                    {line.text}
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </form>
      <section className="project-list">
        <h1>Previous projects</h1>
        {projects.length === 0 ? (
          <p className="empty-state">Projects you open will appear here.</p>
        ) : (
          <div className="project-list__items">
            {projects.map((project) => {
              const armed = pendingDelete === project.id;
              return (
              <div key={project.id} className="project-card">
                <button
                  className="project-card__open"
                  disabled={opening}
                  onClick={() => onOpenRecent(project.id)}
                >
                  <span>{project.display_name}</span>
                  <code>{project.path}</code>
                </button>
                <button
                  className={
                    armed
                      ? "project-card__delete project-card__delete--armed"
                      : "project-card__delete"
                  }
                  title={armed ? "Click again to confirm delete" : `Delete ${project.display_name}`}
                  aria-label={armed ? `Confirm delete ${project.display_name}` : `Delete ${project.display_name}`}
                  disabled={opening}
                  onClick={() => {
                    if (armed) {
                      setPendingDelete(null);
                      onDeleteProject(project.id);
                    } else {
                      setPendingDelete(project.id);
                    }
                  }}
                >
                  {armed ? (
                    "Confirm?"
                  ) : (
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 16 16"
                    fill="currentColor"
                    aria-hidden="true"
                  >
                    <path d="M6 2h4v1h3v1H3V3h3V2zm-3 3h10l-1 9H4L3 5zm3 2v5h1V7H6zm3 0v5h1V7H9z" />
                  </svg>
                  )}
                </button>
              </div>
              );
            })}
          </div>
        )}
      </section>
    </section>
  );
}
