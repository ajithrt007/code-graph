// The graph view. Owns only React Flow concerns: nodes/edges, layout,
// interaction events. Knows nothing about the Tauri backend or domain
// graph types directly — it receives everything via props.

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
  type RFCallEdge,
  type RFMethodNode,
} from "../graph/adapter";
import type { MethodGraph } from "../domain/method";
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
  const baseNodes = useMemo<RFMethodNode[]>(
    () => toReactFlowNodes(graph),
    [graph],
  );
  const baseEdges = useMemo<RFCallEdge[]>(
    () => toReactFlowEdges(graph),
    [graph],
  );

  const { nodes, edges } = useMemo(
    () => applySelection(baseNodes, baseEdges, selectedId, graph),
    [baseNodes, baseEdges, selectedId, graph],
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

  const focusNode = useCallback(
    (id: string, maxZoom = 1) => {
      if (!nodeIdsRef.current.has(id)) return;
      instance?.fitView({
        nodes: [{ id }],
        duration: 300,
        padding: 0.4,
        maxZoom,
      });
      lastCenteredRef.current = id;
    },
    [instance],
  );

  // First sight of a project: top-left-most node, unless a selection was
  // restored (handled by the selection effect below).
  useEffect(() => {
    if (!instance || baseNodes.length === 0) return;
    if (placedRef.current.has(projectId)) return;
    placedRef.current.add(projectId);
    if (!selectedId || !nodeIdsRef.current.has(selectedId)) {
      const top = topLeftMostNodeId(baseNodes);
      if (top) focusNode(top);
    }
  }, [instance, baseNodes, projectId, selectedId, focusNode]);

  // Every selection — explorer, graph node, search, tab switch, restore —
  // brings its node to the center. Deselecting resets so re-selecting the
  // same node after panning away refocuses it.
  useEffect(() => {
    if (!selectedId) {
      lastCenteredRef.current = null;
      return;
    }
    if (selectedId !== lastCenteredRef.current) focusNode(selectedId);
  }, [selectedId, focusNode]);

  useEffect(() => {
    if (focusRequest) focusNode(focusRequest.id, 1.5);
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
