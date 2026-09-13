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
  // the top-left-most node; subsequent graph loads restore the selection.
  const placedRef = useRef<Set<string>>(new Set());
  // Read at load time only so clicks don't yank the viewport.
  const selectedRef = useRef<string | null>(selectedId);
  selectedRef.current = selectedId;
  const nodeIdsRef = useRef<Set<string>>(new Set());
  nodeIdsRef.current = new Set(baseNodes.map((n) => n.id));

  useEffect(() => {
    if (!instance || baseNodes.length === 0) return;
    const alreadyPlaced = placedRef.current.has(projectId);
    const selected = selectedRef.current;
    const target =
      selected && nodeIdsRef.current.has(selected)
        ? selected
        : !alreadyPlaced
          ? topLeftMostNodeId(baseNodes)
          : null;
    if (target) {
      instance.fitView({
        nodes: [{ id: target }],
        duration: 300,
        padding: 0.4,
        maxZoom: 1,
      });
    }
    if (!alreadyPlaced) placedRef.current.add(projectId);
  }, [instance, baseNodes, projectId]);

  useEffect(() => {
    if (focusRequest && nodeIdsRef.current.has(focusRequest.id)) {
      instance?.fitView({
        nodes: [{ id: focusRequest.id }],
        duration: 300,
        padding: 0.25,
        maxZoom: 1.5,
      });
    }
  }, [focusRequest, instance]);

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
