import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { getTerminalOutput } from "../api";
import { loadTerminalReplay } from "../terminalReplay";
import { computeReplayScale, availableContentBox } from "../terminalReplayScale";
import { orkworksTerminalTheme } from "../terminalTheme";
import EmptyState from "./EmptyState";

export default function HistoricalTerminal({ sessionId }: { sessionId: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<"loading" | "empty" | "error" | "loaded">("loading");

  useEffect(() => {
    let current = true;
    let terminal: Terminal | null = null;
    let observer: ResizeObserver | null = null;

    void window.orkworks.getBackendUrl()
      .then((baseUrl) => loadTerminalReplay(
        () => getTerminalOutput(baseUrl, sessionId),
        () => current,
        ({ cols, rows }) => {
          const container = containerRef.current;
          const hasFixedSize = Boolean(cols && rows);
          terminal = new Terminal({
            theme: orkworksTerminalTheme,
            disableStdin: true,
            cursorBlink: false,
            scrollback: 2000,
            ...(hasFixedSize ? { cols, rows } : {}),
          });
          if (!container) return terminal;
          terminal.open(container);

          if (hasFixedSize) {
            // Recorded size known: render at the exact recorded grid (no reflow), then
            // shrink the whole grid with a CSS transform to fit the panel. Everything
            // inside xterm's `.xterm` root is absolutely positioned, so `.xterm` has no
            // natural size of its own — `.xterm-screen` is what xterm sizes in pixels to
            // match cols/rows, so that's what we measure and then stamp onto `.xterm`.
            const xtermEl = container.querySelector<HTMLElement>(".xterm");
            const screenEl = container.querySelector<HTMLElement>(".xterm-screen");
            if (xtermEl && screenEl) {
              let natural: { width: number; height: number } | null = null;

              // Scale against the container's content box, not padding-inclusive
              // clientWidth/clientHeight — xterm lives inside `.terminal-container`'s
              // padding, so the padding-inclusive dimensions would overshoot the scale
              // and `overflow: hidden` would clip the right and bottom edges. The
              // padding math lives in the pure `availableContentBox` helper so it can
              // be unit tested without mounting xterm.
              const readContentBox = () => {
                const cs = window.getComputedStyle(container);
                const padding = {
                  top: parseFloat(cs.paddingTop) || 0,
                  right: parseFloat(cs.paddingRight) || 0,
                  bottom: parseFloat(cs.paddingBottom) || 0,
                  left: parseFloat(cs.paddingLeft) || 0,
                };
                return availableContentBox(
                  { width: container.clientWidth, height: container.clientHeight },
                  padding,
                );
              };

              const applyScale = () => {
                if (!natural) return;
                const scale = computeReplayScale(natural, readContentBox());
                xtermEl.style.transform = `scale(${scale})`;
                xtermEl.style.transformOrigin = "top left";
              };

              const measure = () => {
                if (natural) return;
                const rect = screenEl.getBoundingClientRect();
                // Skip zero-sized measurements (panel hidden or not yet measured) and
                // retry on the next render / container resize. Caching zero would freeze
                // the replay with a 0-px xterm and dispose the render listener, so it
                // would never recover when the panel becomes visible.
                if (rect.width <= 0 || rect.height <= 0) return;
                natural = { width: rect.width, height: rect.height };
                xtermEl.style.width = `${rect.width}px`;
                xtermEl.style.height = `${rect.height}px`;
                renderDisposable.dispose();
                applyScale();
              };

              const renderDisposable = terminal.onRender(measure);
              observer = new ResizeObserver(() => {
                measure();
                applyScale();
              });
              observer.observe(container);
            }
          } else {
            // Legacy sessions with no recorded size: unchanged fit-to-container behavior.
            const fitAddon = new FitAddon();
            terminal.loadAddon(fitAddon);
            try { fitAddon.fit(); } catch { /* container not measured yet */ }
            observer = new ResizeObserver(() => {
              try { fitAddon.fit(); } catch { /* container not measured yet */ }
            });
            observer.observe(container);
          }

          return terminal;
        },
      ))
      .then((result) => {
        if (!current || result === "stale") return;
        setState(result);
      })
      .catch(() => {
        if (current) setState("error");
      });

    return () => {
      current = false;
      observer?.disconnect();
      terminal?.dispose();
    };
  }, [sessionId]);

  if (state === "empty") return <EmptyState message="No saved terminal output for this session." />;
  if (state === "error") return <EmptyState message="Saved terminal output is unavailable." />;
  return <div className="terminal-shell"><div ref={containerRef} className="terminal-container" aria-label={state === "loading" ? "Loading saved terminal output" : "Saved terminal output"} /></div>;
}
