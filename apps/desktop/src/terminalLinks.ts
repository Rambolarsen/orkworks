import type { ILinkHandler } from "@xterm/xterm";

export function terminalLinkHandler(openExternal: (url: string) => Promise<void>): ILinkHandler {
  return {
    activate(_event, url) {
      void openExternal(url).catch((error) => console.error("[terminal] couldn't open external link", error));
    },
  };
}
