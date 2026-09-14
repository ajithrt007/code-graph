// The graph view. Owns only React Flow concerns: nodes/edges, layout,
// interaction events. Knows nothing about the Tauri backend or domain
// graph types directly — it receives everything via props.
//
// The rendered graph is focus-driven: only the focused node's context
// (caller chain up to the roots + focus + immediate callees) is shown,
// derived via `graphOps.subgraphForFocus`. Every selection path
// (explorer, search, graph click, tab restore) funnels through
// `selectedId`, so all of them share this behavior.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactFlow, {
  Background,
  Controls,
  type Edge,
  type Node,
  type NodeMouseHandler,
  type ReactFlowInstance,
} from "reactflow";

import {
  applySelection,
  toReactFlowEdges,
  toReactFlowNodes,
  topLeftMostNodeId,
  type NodeSize,
  type RFCallEdge,
  type RFMethodNode,
} from "../graph/adapter";
import { graphOps, type MethodGraph } from "../domain/method";
import { MethodNode } from "./MethodNode";

const nodeTypes = { method: MethodNode };

interface GraphViewProps {
  projectId: string;
  graph: MethodGraph;
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  /** Center the viewport on this node. `nonce` re-triggers repeat focuses. */
  focusRequest: { id: string; nonce: number } | null;
}

export function GraphView({
  projectId,
  graph,
  selectedId,
  onSelect,
  focusRequest,
}: GraphViewProps) {
  // With no selection (deselected canvas), only the root nodes show with
  // no highlight. The workspace auto-selects the default focus on project
  // load, so first paint already carries a selection.
  // Focus subgraph: recalculated whenever the focus changes. Layout,
  // edges, and highlight all derive from this, never from the full graph.
  const visibleGraph = useMemo(
    () => graphOps.subgraphForFocus(graph, selectedId),
    [graph, selectedId],
  );

  // Measured card sizes reported by React Flow after paint. Feeding them
  // back into the layout upgrades the text-based estimates to actual
  // rendered dimensions, so variable method-name widths cannot overlap.
  const [measuredSizes, setMeasuredSizes] = useState<Map<string, NodeSize>>(
    () => new Map(),
  );
  const measuredRef = useRef(measuredSizes);
  measuredRef.current = measuredSizes;

  const baseNodes = useMemo<RFMethodNode[]>(
    () => toReactFlowNodes(visibleGraph, measuredSizes),
    [visibleGraph, measuredSizes],
  );
  const baseEdges = useMemo<RFCallEdge[]>(
    () => toReactFlowEdges(visibleGraph),
    [visibleGraph],
  );

  const { nodes, edges } = useMemo(
    () => applySelection(baseNodes, baseEdges, selectedId, visibleGraph),
    [baseNodes, baseEdges, selectedId, visibleGraph],
  );

  const handleNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => onSelect(node.id),
    [onSelect],
  );
  const handlePaneClick = useCallback(() => onSelect(null), [onSelect]);

  const [instance, setInstance] = useState<ReactFlowInstance | null>(null);
  // Projects whose initial viewport was already placed. First load centers
  // the top-left-most node; later arrivals center the selection instead.
  const placedRef = useRef<Set<string>>(new Set());
  // Last node the viewport was centered on. Guards the selection/focus
  // effects so one selection triggers exactly one camera move, and lets a
  // re-select of the same node (fresh nonce) refocus after panning away.
  const lastCenteredRef = useRef<string | null>(null);
  const nodeIdsRef = useRef<Set<string>>(new Set());
  nodeIdsRef.current = new Set(baseNodes.map((n) => n.id));

  // Pan the viewport so the node's center is the viewport's center.
  // Uses the measured card size (falls back to the CSS minimums before
  // the first measurement) and preserves the current zoom: focusing is a
  // pure pan, never a zoom jump. Returns whether the node was actually
  // centered — false when React Flow hasn't committed it yet, so callers
  // can retry instead of recording a phantom focus (which used to leave
  // the viewport stranded on a neighboring callee).
  const focusNode = useCallback(
    (id: string): boolean => {
      if (!instance || !nodeIdsRef.current.has(id)) return false;
      const internal = instance.getNode(id);
      if (!internal) return false;
      const width = internal.width ?? 180;
      const height = internal.height ?? 60;
      const base =
        internal.positionAbsolute ?? internal.position ?? { x: 0, y: 0 };
      void instance.setCenter(base.x + width / 2, base.y + height / 2, {
        zoom: instance.getZoom(),
        duration: 300,
      });
      lastCenteredRef.current = id;
      return true;
    },
    [instance],
  );

  // Harvest measured card dimensions after paint and feed them back into
  // the layout. Converges: the re-layout pass reports the same sizes and
  // stops. When sizes actually change, re-center the focused node because
  // its position shifted under the viewport.
  const selectedRef = useRef(selectedId);
  selectedRef.current = selectedId;
  useEffect(() => {
    if (!instance) return;
    const timer = window.setTimeout(() => {
      const internals = instance.getNodes();
      if (internals.length === 0) return;
      const prev = measuredRef.current;
      let changed = false;
      const next = new Map(prev);
      for (const n of internals) {
        if (!n.width || !n.height) continue;
        const old = prev.get(n.id);
        if (!old || old.width !== n.width || old.height !== n.height) {
          next.set(n.id, { width: n.width, height: n.height });
          changed = true;
        }
      }
      if (!changed) return;
      setMeasuredSizes(next);
      // Layout is about to shift: re-center the focused node on the next
      // paint so it stays under the viewport.
      const current = selectedRef.current;
      if (current) {
        window.setTimeout(() => focusNode(current), 60);
      }
    }, 60);
    return () => window.clearTimeout(timer);
  }, [instance, baseNodes, focusNode]);

  // First sight of a project: top-left-most node, unless a selection was
  // restored (handled by the selection effect below). Retries while React
  // Flow is still committing the fresh nodes.
  useEffect(() => {
    if (!instance || baseNodes.length === 0) return;
    if (placedRef.current.has(projectId)) return;
    placedRef.current.add(projectId);
    if (!selectedId || !nodeIdsRef.current.has(selectedId)) {
      const top = topLeftMostNodeId(baseNodes);
      if (!top) return;
      let attempts = 0;
      let timer = 0;
      const attempt = () => {
        if (focusNode(top)) return;
        attempts += 1;
        if (attempts < 12) timer = window.setTimeout(attempt, 80);
      };
      timer = window.setTimeout(attempt, 60);
      return () => window.clearTimeout(timer);
    }
  }, [instance, baseNodes, projectId, selectedId, focusNode]);

  // Every selection — explorer, graph node, search, tab switch, restore —
  // rebuilds the visible subgraph around its node and brings that node
  // itself to the center. Deferred so React Flow has committed the new
  // nodes, keyed on the node set so a focus change recenters even if the
  // id matches, and retried until the node is actually centered (a failed
  // attempt must not count as focused). Deselecting resets to the roots
  // overview; re-selecting the same node after panning away refocuses it.
  useEffect(() => {
    if (!selectedId) {
      lastCenteredRef.current = null;
      return;
    }
    if (selectedId === lastCenteredRef.current) return;
    let attempts = 0;
    let timer = 0;
    const attempt = () => {
      if (focusNode(selectedId)) return;
      attempts += 1;
      if (attempts < 12) timer = window.setTimeout(attempt, 80);
    };
    timer = window.setTimeout(attempt, 60);
    return () => window.clearTimeout(timer);
  }, [selectedId, baseNodes, focusNode]);

  useEffect(() => {
    if (focusRequest) focusNode(focusRequest.id);
  }, [focusRequest, focusNode]);

  return (
    <div className="graph-view">
      <ReactFlow
        nodes={nodes as Node[]}
        edges={edges as Edge[]}
        nodeTypes={nodeTypes}
        onInit={setInstance}
        onNodeClick={handleNodeClick}
        onPaneClick={handlePaneClick}
        minZoom={0.1}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
