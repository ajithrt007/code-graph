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
const message = (value: unknown) =>
  value instanceof Error
    ? value.message
    : typeof value === "string"
      ? value
      : String(value);

export function useWorkspace() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [tabs, setTabs] = useState<ProjectTab[]>([]);
  const [activeTab, setActiveTab] = useState<string>("projects");
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

  const addLoaded = useCallback((loaded: LoadedGraph) => {
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
            selectedId: null,
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
  }, []);

  const openPath = useCallback(
    async (path: string) => {
      try {
        addLoaded(await tauri.openProject(path));
        await reloadProjects();
      } catch (error) {
        window.alert(`Could not open project: ${message(error)}`);
      }
    },
    [addLoaded, reloadProjects],
  );
  const openRecent = useCallback(
    async (projectId: string) => {
      try {
        addLoaded(await tauri.loadProject(projectId));
        await reloadProjects();
      } catch (error) {
        window.alert(`Could not open project: ${message(error)}`);
      }
    },
    [addLoaded, reloadProjects],
  );
  const closeTab = useCallback((projectId: string) => {
    setTabs((current) => current.filter((tab) => tab.id !== projectId));
    setActiveTab((current) => (current === projectId ? "projects" : current));
    void tauri.closeProject(projectId);
  }, []);
  const select = useCallback(
    async (projectId: string, methodId: string | null) => {
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
    setActiveTab,
    openPath,
    openRecent,
    closeTab,
    select,
    toggleEditor,
    saveSource,
  };
}
