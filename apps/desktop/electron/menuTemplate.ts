import type { MenuItemConstructorOptions } from "electron";
import type { AppSettings } from "./settingsMemory";

export interface MenuCommand {
  action: "new-session" | "focus" | "reset-layout";
  panelId?: string;
}

export interface BuildMenuTemplateOptions {
  appName: string;
  platform: NodeJS.Platform;
  settings: AppSettings;
  sendCommand: (command: MenuCommand) => void;
  isHotkeyCaptureActive?: () => boolean;
}

export function buildMenuTemplate(options: BuildMenuTemplateOptions): MenuItemConstructorOptions[] {
  const panelIds = ["sessions", "detail", "terminal", "capacity", "recommendations"] as const;
  const panelTitles: Record<(typeof panelIds)[number], string> = {
    sessions: "Sessions",
    detail: "Detail",
    terminal: "Terminal",
    capacity: "Capacity",
    recommendations: "Recommendations",
  };
  const panelAccelerators: Record<(typeof panelIds)[number], string> = {
    sessions: options.settings.hotkeys.toggleSessionsPanel,
    detail: options.settings.hotkeys.toggleDetailPanel,
    terminal: options.settings.hotkeys.toggleTerminalPanel,
    capacity: options.settings.hotkeys.toggleCapacityPanel,
    recommendations: options.settings.hotkeys.toggleRecommendationsPanel,
  };

  const sendIfNotCapturing = (command: MenuCommand) => {
    if (options.isHotkeyCaptureActive?.()) return;
    options.sendCommand(command);
  };

  const panelItems: MenuItemConstructorOptions[] = panelIds.map((id) => ({
    id,
    label: panelTitles[id],
    accelerator: panelAccelerators[id],
    type: "checkbox",
    checked: true,
    click: () => sendIfNotCapturing({ action: "focus", panelId: id }),
  }));

  const resetLayoutItem: MenuItemConstructorOptions = {
    id: "reset-layout",
    label: "Reset Layout",
    click: () => sendIfNotCapturing({ action: "reset-layout" }),
  };
  if (options.settings.hotkeys.resetLayout) {
    resetLayoutItem.accelerator = options.settings.hotkeys.resetLayout;
  }

  return [
    {
      label: options.appName,
      submenu: [
        { role: "about" },
        { type: "separator" },
        { role: "services" },
        { type: "separator" },
        { role: "hide" },
        { role: "hideOthers" },
        { role: "unhide" },
        { type: "separator" },
        { role: "quit" },
      ],
    },
    {
      label: "File",
      submenu: [
        {
          id: "new-session",
          label: "New Session",
          accelerator: options.settings.hotkeys.newSession,
          click: () => sendIfNotCapturing({ action: "new-session" }),
        },
        { type: "separator" },
        { role: "close" },
      ],
    },
    {
      label: "Edit",
      submenu: [
        { role: "undo" },
        { role: "redo" },
        { type: "separator" },
        { role: "cut" },
        { role: "copy" },
        { role: "paste" },
        { role: "selectAll" },
      ],
    },
    {
      label: "View",
      submenu: [
        ...panelItems,
        { type: "separator" },
        resetLayoutItem,
        { type: "separator" },
        { role: "reload" },
        { role: "forceReload" },
        { role: "toggleDevTools" },
        { type: "separator" },
        { role: "resetZoom" },
        { role: "zoomIn" },
        { role: "zoomOut" },
        { type: "separator" },
        { role: "togglefullscreen" },
      ],
    },
    {
      label: "Window",
      submenu: [
        { role: "minimize" },
        { role: "zoom" },
        ...(options.platform === "darwin"
          ? [{ type: "separator" as const }, { role: "front" as const }]
          : [{ role: "close" as const }]),
      ],
    },
    {
      role: "help",
      submenu: [
        {
          label: "Learn More",
          click: () => {},
        },
      ],
    },
  ];
}

export function findMenuItem(
  template: MenuItemConstructorOptions[],
  id: string,
): MenuItemConstructorOptions | undefined {
  for (const item of template) {
    if (item.id === id) return item;
    if (Array.isArray(item.submenu)) {
      const found = findMenuItem(item.submenu, id);
      if (found) return found;
    }
  }
  return undefined;
}
