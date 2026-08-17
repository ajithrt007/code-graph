// Sidebar with the open-button and a list of the selected method's
// callers/callees. Pure presentation; receives everything via props.

import { useState, type FormEvent } from "react";
import type { MethodNode } from "../domain/method";

interface SidebarProps {
  loadedPath: string | null;
  loading: boolean;
  error: string | null;
  selected: MethodNode | null;
  callers: MethodNode[];
  callees: MethodNode[];
  onAnalyze: (path: string) => void;
  onSelect: (id: string) => void;
}

export function Sidebar(props: SidebarProps) {
  const [path, setPath] = useState("");

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!path.trim()) return;
    props.onAnalyze(path.trim());
  };

  return (
    <aside className="sidebar">
      <header className="sidebar__header">
        <h1>CodeGraph</h1>
        <p className="sidebar__subtitle">Visualize method-level dependencies.</p>
      </header>

      <form className="sidebar__form" onSubmit={submit}>
        <label className="sidebar__label" htmlFor="path">
          .sln, .csproj, or project directory
        </label>
        <input
          id="path"
          className="sidebar__input"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="/path/to/YourSolution.sln"
          disabled={props.loading}
        />
        <button className="sidebar__button" type="submit" disabled={props.loading || !path.trim()}>
          {props.loading ? "Analyzing…" : "Analyze"}
        </button>
      </form>

      {props.error && <div className="sidebar__error">{props.error}</div>}

      {props.loadedPath && (
        <section className="sidebar__section">
          <h2>Loaded</h2>
          <code className="sidebar__path">{props.loadedPath}</code>
        </section>
      )}

      {props.selected && (
        <section className="sidebar__section">
          <h2>Selected</h2>
          <div className="sidebar__selected-name">{props.selected.display_name}</div>
          <code className="sidebar__path">{props.selected.fully_qualified_name}</code>

          <h3>Callers</h3>
          {props.callers.length === 0 ? (
            <p className="sidebar__empty">No callers.</p>
          ) : (
            <ul className="sidebar__list">
              {props.callers.map((m) => (
                <li key={m.id}>
                  <button className="sidebar__list-item" onClick={() => props.onSelect(m.id)}>
                    {m.display_name}
                  </button>
                </li>
              ))}
            </ul>
          )}

          <h3>Callees</h3>
          {props.callees.length === 0 ? (
            <p className="sidebar__empty">No callees.</p>
          ) : (
            <ul className="sidebar__list">
              {props.callees.map((m) => (
                <li key={m.id}>
                  <button className="sidebar__list-item" onClick={() => props.onSelect(m.id)}>
                    {m.display_name}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </aside>
  );
}
