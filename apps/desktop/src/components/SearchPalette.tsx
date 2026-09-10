// Global method search palette (Spotlight-style). Opened with Cmd/Ctrl +
// Shift + F from the project workspace. Queries run against every method's
// source span on the backend; each result shows the method name plus the
// whole matching line. Picking a result selects the method and focuses it
// on the graph.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { tauri } from "../api/tauriClient";
import type { SearchMatch } from "../domain/method";

const DEBOUNCE_MS = 200;

function Highlighted({ text, query }: { text: string; query: string }) {
  const needle = query.trim().toLowerCase();
  if (!needle) return <>{text}</>;
  const lower = text.toLowerCase();
  const parts: ReactNode[] = [];
  let from = 0;
  let key = 0;
  for (;;) {
    const at = lower.indexOf(needle, from);
    if (at === -1) {
      parts.push(text.slice(from));
      break;
    }
    if (at > from) parts.push(text.slice(from, at));
    parts.push(<mark key={key++}>{text.slice(at, at + needle.length)}</mark>);
    from = at + needle.length;
  }
  return <>{parts}</>;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function SearchPalette({
  projectId,
  onPick,
  onClose,
}: {
  projectId: string;
  onPick: (methodId: string) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchMatch[]>([]);
  const [loading, setLoading] = useState(false);
  const [active, setActive] = useState(0);
  const requestId = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!query.trim()) {
      requestId.current += 1;
      setResults([]);
      setLoading(false);
      setActive(0);
      return;
    }
    setLoading(true);
    const id = ++requestId.current;
    const timer = window.setTimeout(() => {
      tauri
        .searchMethods(projectId, query)
        .then((matches) => {
          if (requestId.current !== id) return;
          setResults(matches);
          setActive(0);
          setLoading(false);
        })
        .catch(() => {
          if (requestId.current !== id) return;
          setResults([]);
          setLoading(false);
        });
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query, projectId]);

  const pick = (index: number) => {
    const match = results[index];
    if (match) onPick(match.method.id);
  };

  return (
    <div className="search-overlay" onMouseDown={onClose}>
      <div
        className="search-palette"
        role="dialog"
        aria-label="Search methods"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <input
          ref={inputRef}
          className="search-palette__input"
          type="text"
          placeholder="Search methods…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") onClose();
            else if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((i) => Math.min(i + 1, results.length - 1));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((i) => Math.max(i - 1, 0));
            } else if (event.key === "Enter") {
              pick(active);
            }
          }}
        />
        <div className="search-palette__meta">
          {loading
            ? "Searching…"
            : query.trim()
              ? `${results.length} result${results.length === 1 ? "" : "s"}`
              : "Type to search every method's source"}
        </div>
        <ul className="search-palette__list">
          {results.map((match, index) => (
            <li key={`${match.method.id}:${match.line_number}`}>
              <button
                className={
                  index === active
                    ? "search-palette__item search-palette__item--active"
                    : "search-palette__item"
                }
                onMouseEnter={() => setActive(index)}
                onClick={() => pick(index)}
              >
                <span className="search-palette__name">
                  {match.method.display_name}
                </span>
                <span className="search-palette__line">
                  <span className="search-palette__line-number">
                    {fileName(match.method.file_path)}:{match.line_number}
                  </span>
                  <code>
                    <Highlighted text={match.line_text} query={query} />
                  </code>
                </span>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
