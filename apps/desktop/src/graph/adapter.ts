// Adapter: domain `MethodGraph` -> React Flow `Node[]` / `Edge[]`.
//
// React Flow's `Node`/`Edge` shapes are presentation-layer concerns.
// Components should never construct them inline; they should consume the
// helpers exported here so the boundary stays clean.

import type { CSSProperties } from "react";
import { graphOps } from "../domain/method";
import type { Edge, Node } from "reactflow";
import type { MethodGraph, MethodNode } from "../domain/method";

export interface MethodNodeData extends Record<string, unknown> {
  method: MethodNode;
  /**
   * Visual role used by `MethodNode.tsx` to style the node.
   * - `default`    : outside the selection's impact tree (dimmed on select)
   * - `selected`   : the currently-selected node
   * - `caller`     : calls the selected node, directly or transitively
   *                  (impact chain toward the left-most entry points)
   * - `callee`     : directly called by the selected node
   */
  role: "default" | "selected" | "caller" | "callee";
}

export type NodeRole = MethodNodeData["role"];

export type RFMethodNode = Node<MethodNodeData>;
export type RFCallEdge = Edge;

/**
 * Edge styling per role. React Flow (v11) applies `edge.style` to the edge
 * path, so role-based highlighting lives here rather than in fragile
 * `[data-role]` CSS attribute selectors.
 */
const EDGE_STYLE: Record<NodeRole, CSSProperties> = {
  default: { stroke: "var(--border)", strokeWidth: 1.5, opacity: 0.4 },
  selected: { stroke: "var(--accent)", strokeWidth: 2.5, opacity: 1 },
  caller: { stroke: "var(--accent-caller)", strokeWidth: 2, opacity: 1 },
  callee: { stroke: "var(--accent-callee)", strokeWidth: 2, opacity: 1 },
};

// Tree layout constants: horizontal = call depth (entry points left,
// downstream/external dependencies right), vertical = alphabetical order
// within a level. Spacing sized so node cards (~160px wide, ~60px tall)
// never overlap.
const TREE_H_SPACING = 340;
const TREE_V_SPACING = 120;

/** Opacity applied to nodes/edges outside the selection's impact tree. */
export const DIMMED_NODE_OPACITY = 0.25;
const DIMMED_EDGE_STYLE: CSSProperties = {
  stroke: "var(--border)",
  strokeWidth: 1.5,
  opacity: 0.15,
};

interface ClassNode {
  id: string;
  displayName: string;
  methods: MethodNode[];
  x: number;
  y: number;
}

export interface ClassTreeItem {
  id: string;
  displayName: string;
  methods: MethodNode[];
}

function extractShortClassName(fullyQualifiedName: string): string {
  const parts = fullyQualifiedName.split(".");
  return parts[parts.length - 1] || fullyQualifiedName;
}

function groupByClass(graph: MethodGraph): Map<string, ClassNode> {
  const classes = new Map<string, ClassNode>();
  for (const method of Object.values(graph.methods)) {
    const classId = method.containing_type;
    if (!classes.has(classId)) {
      classes.set(classId, {
        id: classId,
        displayName: extractShortClassName(classId),
        methods: [],
        x: 0,
        y: 0,
      });
    }
    classes.get(classId)!.methods.push(method);
  }
  for (const cls of classes.values()) {
    cls.methods.sort((a, b) => a.location.start_line - b.location.start_line);
  }
  return classes;
}

function buildClassDependencyGraph(
  classes: Map<string, ClassNode>,
  graph: MethodGraph,
): { nodes: Map<string, ClassNode>; edges: Set<string> } {
  const edges = new Set<string>();
  for (const edge of graph.edges) {
    const sourceMethod = graph.methods[edge.source];
    const targetMethod = graph.methods[edge.target];
    if (!sourceMethod || !targetMethod) continue;

    const sourceClass = sourceMethod.containing_type;
    const targetClass = targetMethod.containing_type;
    if (sourceClass !== targetClass) {
      edges.add(`${sourceClass}->${targetClass}`);
    }
  }
  return { nodes: classes, edges };
}

function topologicalSort(depGraph: {
  nodes: Map<string, ClassNode>;
  edges: Set<string>;
}): ClassNode[] {
  const { nodes, edges } = depGraph;
  const inDegree = new Map<string, number>();
  const adjList = new Map<string, string[]>();

  for (const [id] of nodes) {
    inDegree.set(id, 0);
    adjList.set(id, []);
  }
  for (const edge of edges) {
    const [src, tgt] = edge.split("->");
    if (!adjList.has(src) || !adjList.has(tgt)) continue;
    adjList.get(src)!.push(tgt);
    inDegree.set(tgt, (inDegree.get(tgt) ?? 0) + 1);
  }

  const queue = [...nodes.keys()].filter((id) => inDegree.get(id) === 0);
  const result: ClassNode[] = [];

  while (queue.length) {
    const id = queue.shift()!;
    result.push(nodes.get(id)!);
    for (const neighbor of adjList.get(id)!) {
      const deg = inDegree.get(neighbor)! - 1;
      inDegree.set(neighbor, deg);
      if (deg === 0) queue.push(neighbor);
    }
  }

  for (const [id] of nodes) {
    const cls = nodes.get(id)!;
    if (!result.includes(cls)) result.push(cls);
  }
  return result;
}

function compareMethods(a: MethodNode, b: MethodNode): number {
  // Alphabetical by method name (`OrderService.GetOrder(int)` sorts under
  // "GetOrder", not under its class), then display name, then id.
  const byName = (a.name || a.id).toLowerCase().localeCompare(
    (b.name || b.id).toLowerCase(),
  );
  if (byName !== 0) return byName;
  const byDisplay = (a.display_name || "")
    .toLowerCase()
    .localeCompare((b.display_name || "").toLowerCase());
  return byDisplay !== 0 ? byDisplay : a.id.localeCompare(b.id);
}

/**
 * Tree-style layout: depth = shortest call distance from the nearest entry
 * point (in-degree 0). Entry points sit at x=0 (left), downstream
 * dependencies to the right. Nodes sharing a depth are ordered
 * alphabetically by method name. Deliberately ignores containing_type so
 * methods of the same class can spread across levels.
 */
function computeTreePositions(
  graph: MethodGraph,
): Map<string, { x: number; y: number }> {
  const methods = Object.values(graph.methods);
  const positions = new Map<string, { x: number; y: number }>();
  if (methods.length === 0) return positions;

  const outgoing = new Map<string, string[]>();
  const inDegree = new Map<string, number>();
  for (const m of methods) {
    outgoing.set(m.id, []);
    inDegree.set(m.id, 0);
  }
  for (const edge of graph.edges) {
    if (!outgoing.has(edge.source) || !inDegree.has(edge.target)) continue;
    outgoing.get(edge.source)!.push(edge.target);
    inDegree.set(edge.target, (inDegree.get(edge.target) ?? 0) + 1);
  }

  const depth = new Map<string, number>();
  const queue: string[] = [];
  const seedRoots = methods
    .filter((m) => (inDegree.get(m.id) ?? 0) === 0)
    .sort(compareMethods);
  // Fully-cyclic graph: seed from the alphabetically-first node so every
  // node still gets a finite depth.
  const seeds = seedRoots.length > 0 ? seedRoots : [...methods].sort(compareMethods).slice(0, 1);
  for (const root of seeds) {
    depth.set(root.id, 0);
    queue.push(root.id);
  }
  while (queue.length) {
    const id = queue.shift()!;
    const nextDepth = depth.get(id)! + 1;
    for (const target of outgoing.get(id) ?? []) {
      if (depth.has(target)) continue;
      depth.set(target, nextDepth);
      queue.push(target);
    }
  }
  // Disconnected cyclic components unreachable from the seeds form their
  // own left-aligned trees.
  let unvisited = methods.filter((m) => !depth.has(m.id)).sort(compareMethods);
  while (unvisited.length > 0) {
    const root = unvisited[0];
    depth.set(root.id, 0);
    queue.push(root.id);
    while (queue.length) {
      const id = queue.shift()!;
      const nextDepth = depth.get(id)! + 1;
      for (const target of outgoing.get(id) ?? []) {
        if (depth.has(target)) continue;
        depth.set(target, nextDepth);
        queue.push(target);
      }
    }
    unvisited = methods.filter((m) => !depth.has(m.id)).sort(compareMethods);
  }

  const levels = new Map<number, MethodNode[]>();
  for (const m of methods) {
    const d = depth.get(m.id) ?? 0;
    const list = levels.get(d);
    if (list) list.push(m);
    else levels.set(d, [m]);
  }
  for (const [d, list] of levels) {
    list.sort(compareMethods);
    list.forEach((method, index) => {
      positions.set(method.id, {
        x: d * TREE_H_SPACING,
        y: index * TREE_V_SPACING,
      });
    });
  }
  return positions;
}

/** Id of the top-left-most node (min x, then min y). */
export function topLeftMostNodeId(nodes: RFMethodNode[]): string | null {
  let best: RFMethodNode | null = null;
  for (const node of nodes) {
    if (
      !best ||
      node.position.x < best.position.x ||
      (node.position.x === best.position.x &&
        node.position.y < best.position.y)
    ) {
      best = node;
    }
  }
  return best?.id ?? null;
}

/** Ordered class hierarchy shared by the graph layout and left explorer. */
export function orderedClasses(graph: MethodGraph): ClassTreeItem[] {
  const classes = groupByClass(graph);
  return topologicalSort(buildClassDependencyGraph(classes, graph)).map(
    ({ id, displayName, methods }) => ({ id, displayName, methods }),
  );
}

/**
 * Build React Flow nodes from the domain graph using a tree-style layout:
 * - x = call depth (entry points left, downstream dependencies right)
 * - y = alphabetical order of method name within the level
 * Methods of the same class are intentionally NOT pinned to one level.
 */
export function toReactFlowNodes(graph: MethodGraph): RFMethodNode[] {
  const positions = computeTreePositions(graph);

  return Object.values(graph.methods).map((method) => {
    return {
      id: method.id,
      type: "method",
      position: positions.get(method.id) ?? { x: 0, y: 0 },
      data: { method, role: "default" },
    };
  });
}

export function toReactFlowEdges(graph: MethodGraph): RFCallEdge[] {
  return graph.edges.map((edge) => ({
    id: `${edge.source}->${edge.target}`,
    source: edge.source,
    target: edge.target,
    type: "smoothstep",
    animated: false,
    style: EDGE_STYLE.default,
    data: { role: "default", kind: edge.kind },
  }));
}

/**
 * Given a selected method, return new nodes/edges where the selection's
 * impact tree is highlighted and unrelated nodes/edges are de-emphasized.
 *
 * - Callees: direct callees only (unchanged behavior).
 * - Callers: the full recursive chain toward the left-most entry points,
 *   so the potential impact of changing the selected method is visible.
 * - Unrelated nodes/edges get reduced opacity to emphasize the tree.
 *
 * This is the only place selection logic lives; React Flow components
 * just render whatever role they get.
 */
export function applySelection(
  nodes: RFMethodNode[],
  edges: RFCallEdge[],
  selectedId: string | null,
  graph: MethodGraph,
): { nodes: RFMethodNode[]; edges: RFCallEdge[] } {
  if (!selectedId) {
    return { nodes, edges };
  }
  // Recursive impact chain upstream; direct neighbors downstream.
  const callerIds = new Set(
    graphOps.ancestorsOf(graph, selectedId).map((m) => m.id),
  );
  const calleeIds = new Set(
    graphOps.calleesOf(graph, selectedId).map((m) => m.id),
  );
  // Every node that can reach the selection (plus the selection itself)
  // marks the upstream chain; edges inside that set are impact edges.
  const upstream = new Set([selectedId, ...callerIds]);

  const nextNodes = nodes.map((n) => {
    let role: NodeRole = "default";
    if (n.id === selectedId) role = "selected";
    else if (callerIds.has(n.id)) role = "caller";
    else if (calleeIds.has(n.id)) role = "callee";
    const dimmed = role === "default";
    return {
      ...n,
      data: { ...n.data, role },
      style: dimmed ? { opacity: DIMMED_NODE_OPACITY } : { opacity: 1 },
    };
  });

  const nextEdges = edges.map((e) => {
    let role: NodeRole = "default";
    if (upstream.has(e.target) && callerIds.has(e.source)) {
      role = "caller";
    } else if (e.source === selectedId && calleeIds.has(e.target)) {
      role = "callee";
    }
    return {
      ...e,
      data: { ...(e.data ?? {}), role },
      style: role === "default" ? DIMMED_EDGE_STYLE : EDGE_STYLE[role],
    };
  });

  return { nodes: nextNodes, edges: nextEdges };
}
