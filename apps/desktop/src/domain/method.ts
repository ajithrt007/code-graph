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
  source_path: string;
  graph: MethodGraph;
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
};
