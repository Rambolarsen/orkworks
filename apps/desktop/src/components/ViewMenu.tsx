import { useEffect, useRef, useState, type MutableRefObject } from "react";
import type { DockviewApi } from "dockview-react";
import { PANEL_DEFAULTS } from "./DockviewApp";

interface ViewMenuProps {
  dockviewApiRef: MutableRefObject<DockviewApi | null>;
}

export default function ViewMenu({ dockviewApiRef }: ViewMenuProps) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [open]);

  const api = dockviewApiRef.current;

  function togglePanel(id: string) {
    if (!api) return;
    const panel = api.getPanel(id);
    if (panel) {
      panel.api.close();
    } else {
      const def = PANEL_DEFAULTS[id];
      const options: { id: string; component: string; position?: { referencePanel: string; direction: "below" | "right" | "left" | "above" } } = {
        id: def.component,
        component: def.component,
      };
      if (def.position && api.getPanel(def.position.referencePanel)) {
        options.position = {
          referencePanel: def.position.referencePanel,
          direction: def.position.direction,
        };
      }
      api.addPanel(options);
    }
    setOpen(false);
  }

  function resetLayout() {
    if (!api) return;
    api.clear();
    api.addPanel({
      id: PANEL_DEFAULTS.sessions.component,
      component: PANEL_DEFAULTS.sessions.component,
    });
    for (const id of ["detail", "terminal", "capacity", "recommendations"]) {
      const def = PANEL_DEFAULTS[id];
      api.addPanel({
        id: def.component,
        component: def.component,
        position: { referencePanel: def.position!.referencePanel, direction: def.position!.direction },
      });
    }
    setOpen(false);
  }

  return (
    <div className="view-menu" ref={menuRef}>
      <button
        className="view-menu-trigger"
        onClick={() => setOpen(!open)}
      >
        View ▾
      </button>
      {open && (
        <ul className="view-menu-dropdown">
          {Object.entries(PANEL_DEFAULTS).map(([id, def]) => {
            const visible = api ? api.getPanel(def.component) != null : false;
            return (
              <li
                key={id}
                className={visible ? "view-menu-item checked" : "view-menu-item"}
                onClick={() => togglePanel(id)}
              >
                {def.title}
              </li>
            );
          })}
          <li className="view-menu-separator"><hr /></li>
          <li className="view-menu-item view-menu-reset" onClick={resetLayout}>
            Reset Layout
          </li>
        </ul>
      )}
    </div>
  );
}
