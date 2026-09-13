// Frontend-side mirror of the Rust domain types. These shapes are
// produced by the Tauri command layer and are the *only* types that
// components should consume directly. Any React Flow-specific shapes
// are derived from these in the `graph/adapter` module.

export interface SourceLocation {
  file_path: string;
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
}

export interface MethodNode {
  id: string;
  name: string;
  fully_qualified_name: string;
  display_name: string;
  containing_type: string;
  file_path: string;
  location: SourceLocation;
}

export type RelationshipKind = "calls";

export interface CallRelationship {
  source: string;
  target: string;
  kind: RelationshipKind;
}

export interface MethodGraph {
  methods: Record<string, MethodNode>;
  edges: CallRelationship[];
}

export interface LoadedGraph {
  project_id: string;
  source_path: string;
  graph: MethodGraph;
}

export interface ProjectSummary {
  id: string;
  path: string;
  display_name: string;
  last_opened_at: number;
}

export interface MethodSource {
  code: string;
  start_line: number;
}

export interface SearchMatch {
  method: MethodNode;
  line_number: number;
  line_text: string;
}

/**
 * Helpers that operate purely on the domain graph. Kept here so the
 * adapter layer can stay free of business logic.
 */
export const graphOps = {
  callersOf(graph: MethodGraph, targetId: string): MethodNode[] {
    return graph.edges
      .filter((e) => e.target === targetId)
      .map((e) => graph.methods[e.source])
      .filter(Boolean);
  },
  calleesOf(graph: MethodGraph, sourceId: string): MethodNode[] {
    return graph.edges
      .filter((e) => e.source === sourceId)
      .map((e) => graph.methods[e.target])
      .filter(Boolean);
  },
  /**
   * All transitive callers of `targetId` (recursive impact chain toward the
   * left-most entry points). Cycle-safe BFS over reversed edges; the
   * selected node itself is never included even when reachable via a cycle.
   */
  ancestorsOf(graph: MethodGraph, targetId: string): MethodNode[] {
    const byTarget = new Map<string, string[]>();
    for (const e of graph.edges) {
      const list = byTarget.get(e.target);
      if (list) list.push(e.source);
      else byTarget.set(e.target, [e.source]);
    }
    const seen = new Set<string>([targetId]);
    const queue: string[] = [...(byTarget.get(targetId) ?? [])];
    const result: MethodNode[] = [];
    while (queue.length) {
      const id = queue.shift()!;
      if (seen.has(id)) continue;
      seen.add(id);
      const node = graph.methods[id];
      if (node) result.push(node);
      for (const parent of byTarget.get(id) ?? []) {
        if (!seen.has(parent)) queue.push(parent);
      }
    }
    return result;
  },
};
