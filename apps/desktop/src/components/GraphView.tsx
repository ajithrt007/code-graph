// The graph view. Owns only React Flow concerns: nodes/edges, layout,
// interaction events. Knows nothing about the Tauri backend or domain
// graph types directly — it receives everything via props.

import { useCallback, useMemo } from "react";
import ReactFlow, {
  Background,
  Controls,
  type Edge,
  type Node,
  type NodeMouseHandler,
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
}

export function GraphView({ graph, selectedId, onSelect }: GraphViewProps) {
  const baseNodes = useMemo<RFMethodNode[]>(() => toReactFlowNodes(graph), [graph]);
  const baseEdges = useMemo<RFCallEdge[]>(() => toReactFlowEdges(graph), [graph]);

  const { nodes, edges } = useMemo(
    () => applySelection(baseNodes, baseEdges, selectedId, graph),
    [baseNodes, baseEdges, selectedId, graph]
  );

  const handleNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => onSelect(node.id),
    [onSelect]
  );
  const handlePaneClick = useCallback(() => onSelect(null), [onSelect]);

  return (
    <div className="graph-view">
      <ReactFlow
        nodes={nodes as Node[]}
        edges={edges as Edge[]}
        nodeTypes={nodeTypes}
        fitView
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
