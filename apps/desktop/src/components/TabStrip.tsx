interface Tab {
  id: string;
  title: string;
}
export function TabStrip({
  tabs,
  active,
  onActivate,
  onClose,
}: {
  tabs: Tab[];
  active: string;
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
}) {
  return (
    <nav className="tab-strip" aria-label="Open tabs">
      <div
        className={`editor-tab ${active === "projects" ? "editor-tab--active" : ""}`}
      >
        <button
          className="editor-tab__label"
          onClick={() => onActivate("projects")}
        >
          Projects
        </button>
      </div>
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={`editor-tab ${active === tab.id ? "editor-tab--active" : ""}`}
        >
          <button
            className="editor-tab__label"
            onClick={() => onActivate(tab.id)}
          >
            {tab.title}
          </button>
          <button
            className="editor-tab__close"
            aria-label={`Close ${tab.title}`}
            onClick={() => onClose(tab.id)}
          >
            ×
          </button>
        </div>
      ))}
    </nav>
  );
}
