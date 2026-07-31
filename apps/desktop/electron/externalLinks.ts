import type { Shell, WebContents } from "electron";

export function openWebLink(url: string, openExternal: Shell["openExternal"]): void {
  try {
    const protocol = new URL(url).protocol;
    if (protocol === "http:" || protocol === "https:") {
      void openExternal(url).catch((error) => console.error("[main] couldn't open external link", error));
    }
  } catch {}
}

export function configureExternalLinks(webContents: WebContents, openExternal: Shell["openExternal"]): void {
  webContents.setWindowOpenHandler(({ url }) => {
    openWebLink(url, openExternal);
    return { action: "deny" };
  });
  webContents.on("will-navigate", (event, url) => {
    event.preventDefault();
    openWebLink(url, openExternal);
  });
}
