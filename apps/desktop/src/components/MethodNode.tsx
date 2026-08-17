// React Flow custom node for a single method. Pure presentation: receives
// `data` from the adapter and renders.

import { memo } from "react";
import { Handle, Position, type NodeProps } from "reactflow";
import type { MethodNodeData } from "../graph/adapter";

const ROLE_CLASS: Record<MethodNodeData["role"], string> = {
  default: "method-node",
  selected: "method-node method-node--selected",
  caller: "method-node method-node--caller",
  callee: "method-node method-node--callee",
};

function MethodNodeView({ data }: NodeProps<MethodNodeData>) {
  return (
    <div className={ROLE_CLASS[data.role]}>
      <Handle type="target" position={Position.Left} className="method-node__handle" />
      <div className="method-node__type">{data.method.containing_type}</div>
      <div className="method-node__name">{data.method.display_name}</div>
      <Handle type="source" position={Position.Right} className="method-node__handle" />
    </div>
  );
}

export const MethodNode = memo(MethodNodeView);
