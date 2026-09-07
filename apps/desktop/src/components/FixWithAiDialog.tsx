import { useEffect, useRef, useState } from "react";

interface FixWithAiDialogProps {
  initialPrompt: string;
  onConfirm: (prompt: string) => void;
  onCancel: () => void;
}

export default function FixWithAiDialog({ initialPrompt, onConfirm, onCancel }: FixWithAiDialogProps) {
  const [prompt, setPrompt] = useState(initialPrompt);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  useEffect(() => {
    function onDocKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    }
    document.addEventListener("keydown", onDocKeyDown);
    return () => document.removeEventListener("keydown", onDocKeyDown);
  }, [onCancel]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Tab") {
      const container = e.currentTarget as HTMLElement;
      const focusable = container.querySelectorAll<HTMLElement>(
        "textarea:not([disabled]), button:not([disabled])"
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  return (
    <div className="new-session-backdrop" role="presentation" onKeyDown={handleKeyDown}>
      <section className="new-session-dialog" role="dialog" aria-modal="true" aria-labelledby="fix-with-ai-title">
        <header className="new-session-header">
          <h2 id="fix-with-ai-title">Fix with AI</h2>
        </header>

        <div className="new-session-body">
          <div className="new-session-row new-session-row--prompt">
            <label className="new-session-label" htmlFor="fix-with-ai-prompt">Prompt</label>
            <textarea
              ref={textareaRef}
              id="fix-with-ai-prompt"
              className="new-session-textarea"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              rows={8}
            />
          </div>
        </div>

        <footer className="new-session-footer">
          <button type="button" className="new-session-cancel" onClick={onCancel}>Cancel</button>
          <button type="button" className="new-session-confirm" onClick={() => onConfirm(prompt)}>
            Send to session
          </button>
        </footer>
      </section>
    </div>
  );
}
