import Editor from "@monaco-editor/react";
import { useEffect, useRef, useState } from "react";
import type { MethodNode, MethodSource } from "../domain/method";

export function MethodEditor({
  method,
  source,
  methods,
  onSave,
}: {
  method: MethodNode | null;
  source: MethodSource | null;
  methods: MethodNode[];
  onSave: (code: string) => Promise<void>;
}) {
  const saveTimer = useRef<number>();
  const [saving, setSaving] = useState(false);
  useEffect(
    () => () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    },
    [method?.id],
  );
  const change = (code: string | undefined) => {
    if (!code || !method) return;
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(async () => {
      setSaving(true);
      try {
        await onSave(code);
      } finally {
        setSaving(false);
      }
    }, 500);
  };
  return (
    <aside className="method-editor">
      <div className="editor-heading">
        <span>
          {method ? method.fully_qualified_name : "No method selected"}
        </span>
        {saving && <em>Saving…</em>}
      </div>
      {method && source ? (
        <Editor
          height="100%"
          language="csharp"
          theme="vs-dark"
          value={source.code}
          onChange={change}
          beforeMount={(monaco) => {
            monaco.languages.registerCompletionItemProvider("csharp", {
              provideCompletionItems(model, position) {
                const word = model.getWordUntilPosition(position);
                const range = {
                  startLineNumber: position.lineNumber,
                  endLineNumber: position.lineNumber,
                  startColumn: word.startColumn,
                  endColumn: word.endColumn,
                };
                return {
                  suggestions: methods.map((candidate) => ({
                    label: candidate.display_name,
                    kind: monaco.languages.CompletionItemKind.Method,
                    insertText: candidate.name,
                    detail: candidate.fully_qualified_name,
                    range,
                  })),
                };
              },
            });
          }}
          options={{
            readOnly: false,
            minimap: { enabled: false },
            lineNumbers: (value) =>
              String(Number(value) + source.start_line - 1),
            scrollBeyondLastLine: false,
            fontSize: 13,
            padding: { top: 12 },
            wordWrap: "on",
            automaticLayout: true,
          }}
        />
      ) : (
        <div className="editor-empty">
          Select a method in the graph or explorer to inspect and edit its code.
        </div>
      )}
    </aside>
  );
}
