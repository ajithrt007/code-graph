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

/**
 * Build React Flow nodes from the domain graph. Pure: no React Flow state.
 */
export function toReactFlowNodes(graph: MethodGraph): RFMethodNode[] {
  return Object.values(graph.methods).map((method, index) => ({
    id: method.id,
    type: "method",
    // Lay nodes out on a wide grid. Real layouts (dagre / elk) can be
    // slotted in here without touching callers — that's the point of the
    // adapter.
    position: { x: (index % 6) * 240, y: Math.floor(index / 6) * 140 },
    data: { method, role: "default" },
  }));
}

export function toReactFlowEdges(graph: MethodGraph): RFCallEdge[] {
  return graph.edges.map((edge) => ({
    id: `${edge.source}->${edge.target}`,
    source: edge.source,
    target: edge.target,
    label: edge.kind.toUpperCase(),
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
  graph: MethodGraph
): { nodes: RFMethodNode[]; edges: RFCallEdge[] } {
  if (!selectedId) {
    return { nodes, edges };
  }
  const callerIds = new Set(graphOps.callersOf(graph, selectedId).map((m) => m.id));
  const calleeIds = new Set(graphOps.calleesOf(graph, selectedId).map((m) => m.id));

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