import { useState, type FormEvent } from "react";
import type { ProjectSummary } from "../domain/method";
export function ProjectsScreen({
  projects,
  opening,
  onOpenPath,
  onOpenRecent,
}: {
  projects: ProjectSummary[];
  opening: boolean;
  onOpenPath: (path: string) => void;
  onOpenRecent: (id: string) => void;
}) {
  const [path, setPath] = useState("");
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (path.trim() && !opening) onOpenPath(path.trim());
  };
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
        {opening && (
          <p role="status" className="project-opening">
            Opening project… analyzing methods. Large solutions can take a
            while — the app is working, not frozen.
          </p>
        )}
      </form>
      <section className="project-list">
        <h1>Previous projects</h1>
        {projects.length === 0 ? (
          <p className="empty-state">Projects you open will appear here.</p>
        ) : (
            <div className="project-list__items">
              {projects.map((project) => (
                <button
                  key={project.id}
                  className="project-card"
                  disabled={opening}
                  onClick={() => onOpenRecent(project.id)}
                >
                <span>{project.display_name}</span>
                <code>{project.path}</code>
              </button>
            ))}
          </div>
        )}
      </section>
    </section>
  );
}
