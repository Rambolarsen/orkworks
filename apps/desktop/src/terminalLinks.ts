import type { ILinkHandler, ILinkProvider, Terminal } from "@xterm/xterm";

export function terminalLinkHandler(openExternal: (url: string) => Promise<void>): ILinkHandler {
  return {
    activate(_event, url) {
      void openExternal(url).catch((error) => console.error("[terminal] couldn't open external link", error));
    },
  };
}

const PLAN_PATH = /(?:docs\/superpowers\/(?:plans|specs)|specs)\/[\w./-]+\.md\b/g;

export function createTerminalPlanLinkProvider(
  terminal: Terminal,
  onPlanPath: (path: string) => Promise<void>,
): ILinkProvider {
  return {
    provideLinks(y, callback) {
      const line = terminal.buffer.active.getLine(y - 1)?.translateToString(true) ?? "";
      const links = [...line.matchAll(PLAN_PATH)].map((match) => {
        const path = match[0];
        const start = (match.index ?? 0) + 1;
        return {
          text: path,
          range: { start: { x: start, y }, end: { x: start + path.length, y } },
          activate: () => { void onPlanPath(path).catch((error) => console.error("[terminal] couldn't select plan", error)); },
        };
      });
      callback(links.length ? links : undefined);
    },
  };
}
