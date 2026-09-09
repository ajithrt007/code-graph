import { invoke } from "@tauri-apps/api/core";
import type {
  LoadedGraph,
  MethodNode,
  MethodSource,
  ProjectSummary,
} from "../domain/method";

export const tauri = {
  listProjects: () => invoke<ProjectSummary[]>("list_projects"),
  openProject: (path: string) =>
    invoke<LoadedGraph>("open_project", { args: { path } }),
  loadProject: (projectId: string) =>
    invoke<LoadedGraph>("load_project", { args: { project_id: projectId } }),
  refreshProject: (projectId: string) =>
    invoke<LoadedGraph>("refresh_project", { args: { project_id: projectId } }),
  closeProject: (projectId: string) =>
    invoke<void>("close_project", { args: { project_id: projectId } }),
  getMethod: (projectId: string, id: string) =>
    invoke<MethodNode>("get_method", { args: { project_id: projectId, id } }),
  getCallers: (projectId: string, id: string) =>
    invoke<MethodNode[]>("get_callers", {
      args: { project_id: projectId, id },
    }),
  getCallees: (projectId: string, id: string) =>
    invoke<MethodNode[]>("get_callees", {
      args: { project_id: projectId, id },
    }),
  getMethodSource: (projectId: string, id: string) =>
    invoke<MethodSource>("get_method_source", {
      args: { project_id: projectId, id },
    }),
  saveMethodSource: (projectId: string, id: string, code: string) =>
    invoke<void>("save_method_source", {
      args: { project_id: projectId, id, code },
    }),
};
