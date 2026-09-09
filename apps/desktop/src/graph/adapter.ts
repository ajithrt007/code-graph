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
   * - `default`    : not part of the current selection's neighborhood
   * - `selected`   : the currently-selected node
   * - `caller`     : directly calls the selected node
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

// Layout constants
const CLASS_H_SPACING = 350;
const METHOD_V_SPACING = 90;
const CLASS_TOP_MARGIN = 50;

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

function assignPositions(sortedClasses: ClassNode[]): void {
  sortedClasses.forEach((cls, classIndex) => {
    cls.x = classIndex * CLASS_H_SPACING;
    cls.y = CLASS_TOP_MARGIN;
    cls.methods.forEach((method, methodIndex) => {
      (method as MethodNode & { _layout?: { x: number; y: number } })._layout =
        {
          x: cls.x,
          y: cls.y + methodIndex * METHOD_V_SPACING,
        };
    });
  });
}

/** Ordered class hierarchy shared by the graph layout and left explorer. */
export function orderedClasses(graph: MethodGraph): ClassTreeItem[] {
  const classes = groupByClass(graph);
  return topologicalSort(buildClassDependencyGraph(classes, graph)).map(
    ({ id, displayName, methods }) => ({ id, displayName, methods }),
  );
}

/**
 * Build React Flow nodes from the domain graph using hierarchical layout:
 * - Classes ordered left-to-right by call dependency (callers left, callees right)
 * - Methods within a class stacked vertically in source order
 */
export function toReactFlowNodes(graph: MethodGraph): RFMethodNode[] {
  const classes = groupByClass(graph);
  const sortedClasses = topologicalSort(
    buildClassDependencyGraph(classes, graph),
  );
  assignPositions(sortedClasses);

  return Object.values(graph.methods).map((method) => {
    const layout = (
      method as MethodNode & { _layout?: { x: number; y: number } }
    )._layout;
    return {
      id: method.id,
      type: "method",
      position: layout ?? { x: 0, y: 0 },
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
 * neighbors are highlighted and unrelated nodes/edges are de-emphasized.
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
  const callerIds = new Set(
    graphOps.callersOf(graph, selectedId).map((m) => m.id),
  );
  const calleeIds = new Set(
    graphOps.calleesOf(graph, selectedId).map((m) => m.id),
  );

  const nextNodes = nodes.map((n) => {
    let role: NodeRole = "default";
    if (n.id === selectedId) role = "selected";
    else if (callerIds.has(n.id)) role = "caller";
    else if (calleeIds.has(n.id)) role = "callee";
    return { ...n, data: { ...n.data, role } };
  });

  const nextEdges = edges.map((e) => {
    let role: NodeRole = "default";
    if (e.target === selectedId && callerIds.has(e.source)) {
      role = "caller";
    } else if (e.source === selectedId && calleeIds.has(e.target)) {
      role = "callee";
    }
    return { ...e, data: { ...(e.data ?? {}), role }, style: EDGE_STYLE[role] };
  });

  return { nodes: nextNodes, edges: nextEdges };
}
