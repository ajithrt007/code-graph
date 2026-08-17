// Top-level component. Wires the data hook to the sidebar and the graph
// view. No business logic — just composition.

import { GraphView } from "./components/GraphView";
import { Sidebar } from "./components/Sidebar";
import { useGraphData } from "./graph/useGraphData";

export default function App() {
  const { loaded, selectedId, selected, callers, callees, error, loading, analyze, select } =
    useGraphData();

  return (
    <div className="app">
      <Sidebar
        loadedPath={loaded?.source_path ?? null}
        loading={loading}
        error={error}
        selected={selected}
        callers={callers}
        callees={callees}
        onAnalyze={analyze}
        onSelect={(id) => select(id)}
      />
      <main className="app__main">
        {loaded ? (
          <GraphView graph={loaded.graph} selectedId={selectedId} onSelect={select} />
        ) : (
          <div className="app__placeholder">
            <p>Open a .NET solution or project to begin.</p>
          </div>
        )}
      </main>
    </div>
  );
}
