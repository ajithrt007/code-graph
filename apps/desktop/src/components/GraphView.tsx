// The graph view. Owns only React Flow concerns: nodes/edges, layout,
// interaction events. Knows nothing about the Tauri backend or domain
// graph types directly — it receives everything via props.

import { useCallback, useEffect, useMemo, useRef } from "react";
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
  type RFCallEdge,
  type RFMethodNode,
} from "../graph/adapter";
import type { MethodGraph } from "../domain/method";
import { MethodNode } from "./MethodNode";

const nodeTypes = { method: MethodNode };

interface GraphViewProps {
  graph: MethodGraph;
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  /** Center the viewport on this node. `nonce` re-triggers repeat focuses. */
  focusRequest: { id: string; nonce: number } | null;
}

export function GraphView({
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

  const instanceRef = useRef<ReactFlowInstance | null>(null);
  useEffect(() => {
    if (focusRequest) {
      instanceRef.current?.fitView({
        nodes: [{ id: focusRequest.id }],
        duration: 300,
        padding: 0.25,
        maxZoom: 1.5,
      });
    }
  }, [focusRequest]);

  return (
    <div className="graph-view">
      <ReactFlow
        nodes={nodes as Node[]}
        edges={edges as Edge[]}
        nodeTypes={nodeTypes}
        fitView
        onInit={(instance) => {
          instanceRef.current = instance;
        }}
        onNodeClick={handleNodeClick}
        onPaneClick={handlePaneClick}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
