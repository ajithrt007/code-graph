// Hook that owns the loaded graph state. Components subscribe to this
// rather than calling Tauri directly; that keeps the data-fetching
// concern in one place and makes the hook easy to test.

import { useCallback, useEffect, useState } from "react";
import { tauri } from "../api/tauriClient";
import type { LoadedGraph, MethodNode } from "../domain/method";

interface GraphState {
  loaded: LoadedGraph | null;
  selectedId: string | null;
  selected: MethodNode | null;
  callers: MethodNode[];
  callees: MethodNode[];
  error: string | null;
  loading: boolean;
}

/** Convert an unknown `invoke` rejection into a human-readable message. */
function toErrorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "message" in e && typeof e.message === "string") {
    return e.message;
  }
  return String(e);
}

export function useGraphData() {
  const [state, setState] = useState<GraphState>({
    loaded: null,
    selectedId: null,
    selected: null,
    callers: [],
    callees: [],
    error: null,
    loading: false,
  });

  /** Load a new graph by analyzing a path on disk. */
  const analyze = useCallback(async (path: string) => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const loaded = await tauri.analyzeSolution(path);
      setState({
        loaded,
        selectedId: null,
        selected: null,
        callers: [],
        callees: [],
        error: null,
        loading: false,
      });
    } catch (e) {
      setState((s) => ({
        ...s,
        loading: false,
        error: toErrorMessage(e),
      }));
    }
  }, []);

  /** Set or clear the currently-selected method. */
  const select = useCallback(async (id: string | null) => {
    if (id === null) {
      setState((s) => ({
        ...s,
        selectedId: null,
        selected: null,
        callers: [],
        callees: [],
      }));
      return;
    }
    try {
      const [selected, callers, callees] = await Promise.all([
        tauri.getMethod(id),
        tauri.getCallers(id),
        tauri.getCallees(id),
      ]);
      setState((s) => ({ ...s, selectedId: id, selected, callers, callees }));
    } catch (e) {
      setState((s) => ({
        ...s,
        error: toErrorMessage(e),
      }));
    }
  }, []);

  // On mount, if a graph was already loaded (e.g. the user reloaded the
  // window), restore it. Optional — Tauri commands are stateful inside
  // the process so the graph persists for the app's lifetime.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const loaded = await tauri.getGraph();
        if (!cancelled) setState((s) => ({ ...s, loaded }));
      } catch {
        /* no graph loaded yet — ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return { ...state, analyze, select };
}
