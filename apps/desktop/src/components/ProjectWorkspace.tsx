import { useCallback, useEffect, useState } from "react";
import { GraphView } from "./GraphView";
import { ClassExplorer } from "./ClassExplorer";
import { MethodEditor } from "./MethodEditor";
import { SearchPalette } from "./SearchPalette";
import type { ProjectTab } from "../workspace/useWorkspace";

const searchShortcutLabel =
  typeof navigator !== "undefined" && /mac/i.test(navigator.platform)
    ? "⌘⇧F"
    : "Ctrl+Shift+F";

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
  const [searchOpen, setSearchOpen] = useState(false);
  const [focusRequest, setFocusRequest] = useState<{
    id: string;
    nonce: number;
  } | null>(null);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        (event.metaKey || event.ctrlKey) &&
        event.shiftKey &&
        event.code === "KeyF"
      ) {
        const target = event.target as HTMLElement | null;
        if (
          target &&
          (target.tagName === "INPUT" ||
            target.tagName === "TEXTAREA" ||
            target.tagName === "SELECT" ||
            target.isContentEditable)
        ) {
          return;
        }
        event.preventDefault();
        setSearchOpen(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // Every selection — explorer, graph node, or search — both selects the
  // method (editor + highlight) and focuses its graph node. The nonce
  // re-triggers focus even when re-selecting the already-selected node.
  const handleSelect = useCallback(
    (methodId: string | null) => {
      onSelect(methodId);
      if (methodId) setFocusRequest({ id: methodId, nonce: Date.now() });
    },
    [onSelect],
  );

  const handleSearchPick = (methodId: string) => {
    setSearchOpen(false);
    handleSelect(methodId);
  };

  return (
    <section className="project-workspace">
      {searchOpen && (
        <SearchPalette
          projectId={tab.loaded.project_id}
          onPick={handleSearchPick}
          onClose={() => setSearchOpen(false)}
        />
      )}
      <ClassExplorer
        graph={tab.loaded.graph}
        selectedId={tab.selectedId}
        onSelect={handleSelect}
      />
      <main className="project-workspace__graph">
        <GraphView
          projectId={tab.loaded.project_id}
          graph={tab.loaded.graph}
          selectedId={tab.selectedId}
          onSelect={handleSelect}
          focusRequest={focusRequest}
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
        <button
          title={`Search methods (${searchShortcutLabel})`}
          onClick={() => setSearchOpen(true)}
        >
          Search {searchShortcutLabel}
        </button>
        <button title="Toggle code editor" onClick={onToggleEditor}>
          ‹›
        </button>
      </footer>
    </section>
  );
}
