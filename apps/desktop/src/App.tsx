import { ProjectsScreen } from "./components/ProjectsScreen";
import { ProjectWorkspace } from "./components/ProjectWorkspace";
import { TabStrip } from "./components/TabStrip";
import { useWorkspace } from "./workspace/useWorkspace";

export default function App() {
  const workspace = useWorkspace();
  const active = workspace.tabs.find((tab) => tab.id === workspace.activeTab);
  return (
    <div className="app">
      <TabStrip
        tabs={workspace.tabs}
        active={workspace.activeTab}
        onActivate={workspace.setActiveTab}
        onClose={workspace.closeTab}
      />
      {workspace.activeTab === "projects" ? (
        <ProjectsScreen
          projects={workspace.projects}
          onOpenPath={workspace.openPath}
          onOpenRecent={workspace.openRecent}
        />
      ) : active ? (
        <ProjectWorkspace
          tab={active}
          onSelect={(id) => workspace.select(active.id, id)}
          onToggleEditor={() => workspace.toggleEditor(active.id)}
          onSave={(methodId, code) =>
            workspace.saveSource(active.id, methodId, code)
          }
        />
      ) : null}
    </div>
  );
}
