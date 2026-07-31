import type { Shell, WebContents } from "electron";

export function openWebLink(url: string, openExternal: Shell["openExternal"]): void {
  try {
    const protocol = new URL(url).protocol;
    if (protocol === "http:" || protocol === "https:") {
      void openExternal(url).catch((error) => console.error("[main] couldn't open external link", error));
    }
  } catch {
    // Invalid URLs remain denied.
  }
}

function isAllowedNavigation(url: string, allowedOrigin?: string): boolean {
  try {
    return allowedOrigin !== undefined && new URL(url).origin === new URL(allowedOrigin).origin;
  } catch {
    return false;
  }
}

export function configureExternalLinks(webContents: WebContents, openExternal: Shell["openExternal"], allowedOrigin?: string): void {
  webContents.setWindowOpenHandler(({ url }) => {
    openWebLink(url, openExternal);
    return { action: "deny" };
  });
  webContents.on("will-navigate", (event, url) => {
    if (isAllowedNavigation(url, allowedOrigin)) return;
    event.preventDefault();
    openWebLink(url, openExternal);
  });
}
