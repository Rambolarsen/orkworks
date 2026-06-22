import type { MenuItemConstructorOptions } from "electron";
import type { AppSettings } from "./settingsMemory";

export interface MenuCommand {
  action: "new-session" | "focus" | "reset-layout" | "open-settings";
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
  const isCapturing = options.isHotkeyCaptureActive?.() ?? false;
  const panelIds = ["sessions", "detail", "terminal", "capacity", "recommendations"] as const;
  const panelTitles: Record<(typeof panelIds)[number], string> = {
    sessions: "Sessions",
    detail: "Detail",
    terminal: "Terminal",
    capacity: "Providers",
    recommendations: "Recommendations",
  };
  const panelAccelerators: Record<(typeof panelIds)[number], string> = {
    sessions: options.settings.hotkeys.toggleSessionsPanel,
    detail: options.settings.hotkeys.toggleDetailPanel,
    terminal: options.settings.hotkeys.toggleTerminalPanel,
    capacity: options.settings.hotkeys.toggleCapacityPanel,
    recommendations: options.settings.hotkeys.toggleRecommendationsPanel,
  };

  const acceleratorUnlessCapturing = (accelerator: string | null): string | undefined =>
    isCapturing ? undefined : (accelerator ?? undefined);
  const nativeRole = (role: NonNullable<MenuItemConstructorOptions["role"]>, label: string): MenuItemConstructorOptions =>
    isCapturing ? { label, enabled: false } : { role };
  const sendIfNotCapturing = (command: MenuCommand) => {
    if (isCapturing) return;
    options.sendCommand(command);
  };

  const panelItems: MenuItemConstructorOptions[] = panelIds.map((id) => ({
    id,
    label: panelTitles[id],
    accelerator: acceleratorUnlessCapturing(panelAccelerators[id]),
    type: "checkbox",
    checked: true,
    enabled: !isCapturing,
    click: () => sendIfNotCapturing({ action: "focus", panelId: id }),
  }));

  const resetLayoutItem: MenuItemConstructorOptions = {
    id: "reset-layout",
    label: "Reset Layout",
    click: () => sendIfNotCapturing({ action: "reset-layout" }),
  };
  const resetLayoutAccelerator = acceleratorUnlessCapturing(options.settings.hotkeys.resetLayout);
  if (resetLayoutAccelerator) {
    resetLayoutItem.accelerator = resetLayoutAccelerator;
  }

  return [
    {
      label: options.appName,
      submenu: [
        nativeRole("about", `About ${options.appName}`),
        { type: "separator" },
        nativeRole("services", "Services"),
        { type: "separator" },
        nativeRole("hide", `Hide ${options.appName}`),
        nativeRole("hideOthers", "Hide Others"),
        nativeRole("unhide", "Show All"),
        { type: "separator" },
        nativeRole("quit", `Quit ${options.appName}`),
      ],
    },
    {
      label: "File",
      submenu: [
        {
          id: "new-session",
          label: "New Session",
          accelerator: acceleratorUnlessCapturing(options.settings.hotkeys.newSession),
          click: () => sendIfNotCapturing({ action: "new-session" }),
        },
        { type: "separator" },
        nativeRole("close", "Close"),
      ],
    },
    {
      label: "Edit",
      submenu: [
        nativeRole("undo", "Undo"),
        nativeRole("redo", "Redo"),
        { type: "separator" },
        nativeRole("cut", "Cut"),
        nativeRole("copy", "Copy"),
        nativeRole("paste", "Paste"),
        nativeRole("selectAll", "Select All"),
      ],
    },
    {
      label: "View",
      submenu: [
        ...panelItems,
        { type: "separator" },
        resetLayoutItem,
        { type: "separator" },
        nativeRole("reload", "Reload"),
        nativeRole("forceReload", "Force Reload"),
        nativeRole("toggleDevTools", "Toggle Developer Tools"),
        { type: "separator" },
        nativeRole("resetZoom", "Actual Size"),
        nativeRole("zoomIn", "Zoom In"),
        nativeRole("zoomOut", "Zoom Out"),
        { type: "separator" },
        nativeRole("togglefullscreen", "Toggle Full Screen"),
      ],
    },
    {
      label: "Window",
      submenu: [
        nativeRole("minimize", "Minimize"),
        nativeRole("zoom", "Zoom"),
        ...(options.platform === "darwin"
          ? [{ type: "separator" as const }, nativeRole("front", "Bring All to Front")]
          : [nativeRole("close", "Close")]),
      ],
    },
    {
      ...(isCapturing ? { label: "Help" } : { role: "help" as const }),
      submenu: [
        {
          id: "open-settings",
          label: "Settings…",
          accelerator: acceleratorUnlessCapturing("CmdOrCtrl+,"),
          click: () => sendIfNotCapturing({ action: "open-settings" }),
        },
        { type: "separator" },
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
