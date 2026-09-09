import { useEffect, useMemo, useState } from "react";
import { orderedClasses } from "../graph/adapter";
import type { MethodGraph } from "../domain/method";

export function ClassExplorer({
  graph,
  selectedId,
  onSelect,
}: {
  graph: MethodGraph;
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  useEffect(() => {
    if (!selectedId) return;
    const selected = graph.methods[selectedId];
    if (selected)
      setExpanded((current) => new Set(current).add(selected.containing_type));
  }, [graph, selectedId]);
  const needle = query.trim().toLocaleLowerCase();
  const classes = useMemo(
    () =>
      orderedClasses(graph).map((cls) => ({
        ...cls,
        methods: cls.methods.filter(
          (method) =>
            !needle ||
            method.display_name.toLowerCase().includes(needle) ||
            method.name.toLowerCase().includes(needle),
        ),
        classMatches: cls.displayName.toLowerCase().includes(needle),
      })),
    [graph, needle],
  );
  const visible = classes.filter(
    (cls) => !needle || cls.classMatches || cls.methods.length,
  );
  return (
    <aside className="class-explorer">
      <div className="explorer-title">EXPLORER</div>
      <input
        className="explorer-search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Search classes and methods"
      />
      <div className="class-tree">
        {visible.map((cls) => {
          const open = needle ? true : expanded.has(cls.id);
          return (
            <div key={cls.id} className="class-tree__class">
              <button
                className="class-tree__class-row"
                onClick={() =>
                  setExpanded((current) => {
                    const next = new Set(current);
                    next.has(cls.id) ? next.delete(cls.id) : next.add(cls.id);
                    return next;
                  })
                }
              >
                <span className="class-tree__chevron">{open ? "⌄" : "›"}</span>
                {cls.displayName}
              </button>
              {open &&
                cls.methods.map((method) => (
                  <button
                    key={method.id}
                    className={`class-tree__method ${selectedId === method.id ? "class-tree__method--selected" : ""}`}
                    onClick={() => onSelect(method.id)}
                  >
                    {method.display_name}
                  </button>
                ))}
            </div>
          );
        })}
        {visible.length === 0 && (
          <p className="explorer-empty">No matching names.</p>
        )}
      </div>
    </aside>
  );
}
