// Tauri IPC client. The single place where the frontend calls into the
// backend. Every other module receives `LoadedGraph`/`MethodNode`
// values through these functions and treats them as ordinary TS values —
// no `invoke` leakage past this file.

import { invoke } from "@tauri-apps/api/core";
import type { LoadedGraph, MethodNode } from "../domain/method";

interface AnalyzeArgs {
  path: string;
}

interface MethodArgs {
  id: string;
}

export const tauri = {
  analyzeSolution(path: string): Promise<LoadedGraph> {
    return invoke<LoadedGraph>("analyze_solution", { args: { path } satisfies AnalyzeArgs });
  },

  getGraph(): Promise<LoadedGraph> {
    return invoke<LoadedGraph>("get_graph");
  },

  getMethod(id: string): Promise<MethodNode> {
    return invoke<MethodNode>("get_method", { args: { id } satisfies MethodArgs });
  },

  getCallers(id: string): Promise<MethodNode[]> {
    return invoke<MethodNode[]>("get_callers", { args: { id } satisfies MethodArgs });
  },

  getCallees(id: string): Promise<MethodNode[]> {
    return invoke<MethodNode[]>("get_callees", { args: { id } satisfies MethodArgs });
  },
};
