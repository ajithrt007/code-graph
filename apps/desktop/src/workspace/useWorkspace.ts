import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { tauri } from "../api/tauriClient";
import type {
  LoadedGraph,
  MethodSource,
  ProjectSummary,
} from "../domain/method";

export interface ProjectTab {
  id: string;
  title: string;
  loaded: LoadedGraph;
  selectedId: string | null;
  source: MethodSource | null;
  rightOpen: boolean;
  loading: boolean;
  error: string | null;
}

/** One line in the project-loading terminal. */
export interface OpenLogLine {
  time: string;
  text: string;
}

/** Backend `project-load-progress` event payload. */
interface LoadProgressEvent {
  target: string;
  stage: string;
  detail: string;
  current: number;
  total: number;
}

const MAX_LOG_LINES = 200;

/** Last selected/focused method per project, restored on subsequent loads. */
const LAST_SELECTED_PREFIX = "codegraph:lastSelected:";
const readLastSelected = (projectId: string): string | null => {
  try {
    if (typeof window === "undefined") return null;
    return window.localStorage.getItem(LAST_SELECTED_PREFIX + projectId);
  } catch {
    return null;
  }
};
const writeLastSelected = (projectId: string, methodId: string): void => {
  try {
    window.localStorage.setItem(LAST_SELECTED_PREFIX + projectId, methodId);
  } catch {
    // Storage unavailable (private mode, tests) — viewport restore skips.
  }
};
const clearLastSelected = (projectId: string): void => {
  try {
    window.localStorage.removeItem(LAST_SELECTED_PREFIX + projectId);
  } catch {
    // Ignore.
  }
};
const message = (value: unknown): string => {
  if (value instanceof Error) return value.message;
  if (typeof value === "string") return value;
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (typeof record.message === "string" && record.message) {
      return record.message;
    }
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
};

export function useWorkspace() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [tabs, setTabs] = useState<ProjectTab[]>([]);
  const [activeTab, setActiveTab] = useState<string>("projects");
  const [opening, setOpening] = useState(false);
  // Mini-terminal log fed by backend `project-load-progress` events.
  const [openLog, setOpenLog] = useState<OpenLogLine[]>([]);
  const [openProgress, setOpenProgress] = useState<{
    current: number;
    total: number;
    stage: string;
  } | null>(null);
  const [openFailed, setOpenFailed] = useState(false);
  const openingRef = useRef(false);
  const timers = useRef(new Map<string, number>());
  const reloadProjects = useCallback(
    () =>
      tauri
        .listProjects()
        .then(setProjects)
        .catch(() => undefined),
    [],
  );
  useEffect(() => {
    void reloadProjects();
  }, [reloadProjects]);

  const appendLog = useCallback((text: string) => {
    const line: OpenLogLine = {
      time: new Date().toLocaleTimeString(),
      text,
    };
    setOpenLog((current) => [...current.slice(-(MAX_LOG_LINES - 1)), line]);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<LoadProgressEvent>("project-load-progress", (event) => {
      if (!openingRef.current) return;
      appendLog(event.payload.detail);
      setOpenProgress({
        current: event.payload.current,
        total: event.payload.total,
        stage: event.payload.stage,
      });
    }).then((stop) => {
      unlisten = stop;
    });
    return () => {
      unlisten?.();
    };
  }, [appendLog]);

  const beginOpen = useCallback(() => {
    setOpenLog([]);
    setOpenProgress(null);
    setOpenFailed(false);
    setOpening(true);
    openingRef.current = true;
  }, []);

  const endOpen = useCallback(() => {
    setOpening(false);
    openingRef.current = false;
  }, []);

  const dismissOpenLog = useCallback(() => {
    setOpenLog([]);
    setOpenProgress(null);
    setOpenFailed(false);
  }, []);

  const addLoaded = useCallback((loaded: LoadedGraph) => {
    const stored = readLastSelected(loaded.project_id);
    const restoredId =
      stored && loaded.graph.methods[stored] ? stored : null;
    setTabs((current) => {
      const title =
        loaded.source_path
          .split(/[\\/]/)
          .pop()
          ?.replace(/\.(sln|csproj)$/i, "") || "Project";
      const existing = current.find((tab) => tab.id === loaded.project_id);
      const next: ProjectTab = existing
        ? { ...existing, loaded, loading: false, error: null }
        : {
            id: loaded.project_id,
            title,
            loaded,
            selectedId: restoredId,
            source: null,
            rightOpen: true,
            loading: false,
            error: null,
          };
      return existing
        ? current.map((tab) => (tab.id === next.id ? next : tab))
        : [...current, next];
    });
    setActiveTab(loaded.project_id);
    if (restoredId) {
      void tauri
        .getMethodSource(loaded.project_id, restoredId)
        .then((source) => {
          setTabs((current) =>
            current.map((tab) =>
              tab.id === loaded.project_id &&
              tab.selectedId === restoredId
                ? { ...tab, source }
                : tab,
            ),
          );
        })
        .catch(() => undefined);
    }
  }, []);

  const openPath = useCallback(
    async (path: string) => {
      beginOpen();
      try {
        addLoaded(await tauri.openProject(path));
        await reloadProjects();
      } catch (error) {
        const text = message(error);
        appendLog(`Failed: ${text}`);
        setOpenFailed(true);
        window.alert(`Could not open project: ${text}`);
      } finally {
        endOpen();
      }
    },
    [addLoaded, appendLog, beginOpen, endOpen, reloadProjects],
  );
  const openRecent = useCallback(
    async (projectId: string) => {
      beginOpen();
      try {
        addLoaded(await tauri.loadProject(projectId));
        await reloadProjects();
      } catch (error) {
        const text = message(error);
        appendLog(`Failed: ${text}`);
        setOpenFailed(true);
        window.alert(`Could not open project: ${text}`);
      } finally {
        endOpen();
      }
    },
    [addLoaded, appendLog, beginOpen, endOpen, reloadProjects],
  );
  const closeTab = useCallback((projectId: string) => {
    setTabs((current) => current.filter((tab) => tab.id !== projectId));
    setActiveTab((current) => (current === projectId ? "projects" : current));
    void tauri.closeProject(projectId);
  }, []);
  const deleteProject = useCallback(
    async (projectId: string) => {
      try {
        await tauri.deleteProject(projectId);
        closeTab(projectId);
        clearLastSelected(projectId);
        await reloadProjects();
      } catch (error) {
        window.alert(`Could not delete project: ${message(error)}`);
      }
    },
    [closeTab, reloadProjects],
  );
  const select = useCallback(
    async (projectId: string, methodId: string | null) => {
      if (methodId) writeLastSelected(projectId, methodId);
      setTabs((current) =>
        current.map((tab) =>
          tab.id === projectId
            ? { ...tab, selectedId: methodId, source: null }
            : tab,
        ),
      );
      if (!methodId) return;
      try {
        const source = await tauri.getMethodSource(projectId, methodId);
        setTabs((current) =>
          current.map((tab) =>
            tab.id === projectId && tab.selectedId === methodId
              ? { ...tab, source }
              : tab,
          ),
        );
      } catch (error) {
        setTabs((current) =>
          current.map((tab) =>
            tab.id === projectId ? { ...tab, error: message(error) } : tab,
          ),
        );
      }
    },
    [],
  );
  const toggleEditor = useCallback((projectId: string) => {
    setTabs((current) =>
      current.map((tab) =>
        tab.id === projectId ? { ...tab, rightOpen: !tab.rightOpen } : tab,
      ),
    );
  }, []);
  const saveSource = useCallback(
    (projectId: string, methodId: string, code: string) => {
      setTabs((current) =>
        current.map((tab) =>
          tab.id === projectId && tab.selectedId === methodId && tab.source
            ? { ...tab, source: { ...tab.source, code } }
            : tab,
        ),
      );
      return tauri
        .saveMethodSource(projectId, methodId, code)
        .catch((error) => {
          setTabs((current) =>
            current.map((tab) =>
              tab.id === projectId ? { ...tab, error: message(error) } : tab,
            ),
          );
          throw error;
        });
    },
    [],
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("project-files-changed", (event) => {
      const id = event.payload;
      if (timers.current.has(id)) window.clearTimeout(timers.current.get(id));
      const timer = window.setTimeout(async () => {
        timers.current.delete(id);
        setTabs((current) =>
          current.map((tab) =>
            tab.id === id ? { ...tab, loading: true, error: null } : tab,
          ),
        );
        try {
          const loaded = await tauri.refreshProject(id);
          setTabs((current) =>
            current.map((tab) => {
              if (tab.id !== id) return tab;
              const selectedId =
                tab.selectedId && loaded.graph.methods[tab.selectedId]
                  ? tab.selectedId
                  : null;
              return {
                ...tab,
                loaded,
                selectedId,
                source: selectedId ? tab.source : null,
                loading: false,
              };
            }),
          );
        } catch (error) {
          setTabs((current) =>
            current.map((tab) =>
              tab.id === id
                ? { ...tab, loading: false, error: message(error) }
                : tab,
            ),
          );
        }
      }, 600);
      timers.current.set(id, timer);
    }).then((stop) => {
      unlisten = stop;
    });
    return () => {
      unlisten?.();
      timers.current.forEach((timer) => window.clearTimeout(timer));
    };
  }, []);

  return {
    projects,
    tabs,
    activeTab,
    opening,
    openLog,
    openProgress,
    openFailed,
    setActiveTab,
    openPath,
    openRecent,
    closeTab,
    deleteProject,
    dismissOpenLog,
    select,
    toggleEditor,
    saveSource,
  };
}
