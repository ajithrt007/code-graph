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
  /**
   * All transitive callees of `sourceId`. Cycle-safe BFS over forward
   * edges; the source node itself is never included even when reachable
   * via a cycle.
   */
  descendantsOf(graph: MethodGraph, sourceId: string): MethodNode[] {
    const bySource = new Map<string, string[]>();
    for (const e of graph.edges) {
      const list = bySource.get(e.source);
      if (list) list.push(e.target);
      else bySource.set(e.source, [e.target]);
    }
    const seen = new Set<string>([sourceId]);
    const queue: string[] = [...(bySource.get(sourceId) ?? [])];
    const result: MethodNode[] = [];
    while (queue.length) {
      const id = queue.shift()!;
      if (seen.has(id)) continue;
      seen.add(id);
      const node = graph.methods[id];
      if (node) result.push(node);
      for (const child of bySource.get(id) ?? []) {
        if (!seen.has(child)) queue.push(child);
      }
    }
    return result;
  },
  /**
   * Ids of the left-most/root nodes: methods with no callers inside the
   * graph (in-degree 0). Sorted alphabetically by method name so the
   * order is deterministic.
   */
  rootIds(graph: MethodGraph): string[] {
    const targeted = new Set(graph.edges.map((e) => e.target));
    return Object.values(graph.methods)
      .filter((m) => !targeted.has(m.id))
      .sort(compareMethodOrder)
      .map((m) => m.id);
  },
  /**
   * Default focus for a freshly opened project: the entry-point root —
   * the root node reaching the most downstream methods. A real entry
   * point (e.g. `Main`) reaches most of the graph, while an uncalled
   * leaf such as an unused `FindById(int, bool)` overload reaches
   * nothing and must not win on alphabetical order alone. Ties (and the
   * edgeless single-node graph) fall back to alphabetical root order.
   * Null for an empty graph.
   */
  defaultFocusId(graph: MethodGraph): string | null {
    const roots = graphOps.rootIds(graph);
    if (roots.length === 0) return null;
    let best = roots[0];
    let bestReach = -1;
    for (const id of roots) {
      const reach = graphOps.descendantsOf(graph, id).length;
      if (reach > bestReach) {
        bestReach = reach;
        best = id;
      }
    }
    return best;
  },
  /**
   * Focus-driven visible set. For any focused node:
   *
   *   visible = the focus itself
   *           + its caller chain all the way up to the left-most/root nodes
   *           + its immediate callees
   *
   * Everything else is excluded, so selecting another node clears the
   * previous context entirely. With no focus (deselected canvas), only
   * the root nodes show as a neutral overview.
   */
  focusVisibleIds(graph: MethodGraph, focusedId: string | null): Set<string> {
    if (!focusedId || !graph.methods[focusedId]) {
      return new Set<string>(graphOps.rootIds(graph));
    }
    const visible = new Set<string>([focusedId]);
    for (const ancestor of graphOps.ancestorsOf(graph, focusedId)) {
      visible.add(ancestor.id);
    }
    for (const callee of graphOps.calleesOf(graph, focusedId)) {
      visible.add(callee.id);
    }
    return visible;
  },
  /**
   * The focus subgraph: visible nodes plus every edge whose endpoints are
   * both visible. Edges along the caller chain and focus->callee edges
   * survive automatically; unrelated edges drop out with their nodes.
   */
  subgraphForFocus(graph: MethodGraph, focusedId: string | null): MethodGraph {
    const visible = graphOps.focusVisibleIds(graph, focusedId);
    const methods: Record<string, MethodNode> = {};
    for (const id of visible) {
      const node = graph.methods[id];
      if (node) methods[id] = node;
    }
    return {
      methods,
      edges: graph.edges.filter((e) => visible.has(e.source) && visible.has(e.target)),
    };
  },
};

/**
 * Deterministic method order: alphabetical by method name
 * (`OrderService.GetOrder(int)` sorts under "GetOrder"), then display
 * name, then id. Shared by root ordering and the graph layout so the
 * "top-most" node is the same everywhere.
 */
export function compareMethodOrder(a: MethodNode, b: MethodNode): number {
  const byName = (a.name || a.id).toLowerCase().localeCompare(
    (b.name || b.id).toLowerCase(),
  );
  if (byName !== 0) return byName;
  const byDisplay = (a.display_name || "")
    .toLowerCase()
    .localeCompare((b.display_name || "").toLowerCase());
  return byDisplay !== 0 ? byDisplay : a.id.localeCompare(b.id);
}
