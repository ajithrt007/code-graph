import { GraphView } from "./GraphView";
import { ClassExplorer } from "./ClassExplorer";
import { MethodEditor } from "./MethodEditor";
import type { ProjectTab } from "../workspace/useWorkspace";

export function ProjectWorkspace({
  tab,
  onSelect,
  onToggleEditor,
  onSave,
}: {
  tab: ProjectTab;
  onSelect: (id: string | null) => void;
  onToggleEditor: () => void;
  onSave: (methodId: string, code: string) => Promise<void>;
}) {
  const selected = tab.selectedId
    ? (tab.loaded.graph.methods[tab.selectedId] ?? null)
    : null;
  return (
    <section className="project-workspace">
      <ClassExplorer
        graph={tab.loaded.graph}
        selectedId={tab.selectedId}
        onSelect={onSelect}
      />
      <main className="project-workspace__graph">
        <GraphView
          graph={tab.loaded.graph}
          selectedId={tab.selectedId}
          onSelect={onSelect}
        />
        {tab.loading && (
          <div className="refresh-indicator">Refreshing graph…</div>
        )}
        {tab.error && <div className="workspace-error">{tab.error}</div>}
      </main>
      {tab.rightOpen && (
        <MethodEditor
          method={selected}
          source={tab.source}
          methods={Object.values(tab.loaded.graph.methods)}
          onSave={(code) => onSave(selected!.id, code)}
        />
      )}
      <footer className="bottom-bar">
        <span>{tab.title}</span>
        <button title="Toggle code editor" onClick={onToggleEditor}>
          ‹›
        </button>
      </footer>
    </section>
  );
}
