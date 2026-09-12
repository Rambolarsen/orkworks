import type { BrowserWindowConstructorOptions } from "electron";

export function getWindowChromeOptions(platform: NodeJS.Platform): BrowserWindowConstructorOptions {
  if (platform === "win32") {
    return {
      titleBarStyle: "hidden",
      autoHideMenuBar: true,
      // Match the app's dark tokens, independently of the OS theme.
      titleBarOverlay: { color: "#0c0d10", symbolColor: "#eceef1", height: 38 },
    };
  }
  return platform === "darwin" ? { titleBarStyle: "hiddenInset" } : {};
}
