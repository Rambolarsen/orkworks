import type { BrowserWindow, IpcMain, IpcMainInvokeEvent, Notification, NotificationConstructorOptions } from "electron";

interface NotificationDependencies {
  ipc: Pick<IpcMain, "handle">;
  isSupported: () => boolean;
  create: (options: NotificationConstructorOptions) => Pick<Notification, "once" | "show" | "close">;
}

export function installSessionNotificationIpc(
  getWindow: () => BrowserWindow | null,
  enabled: () => boolean,
  { ipc, isSupported, create }: NotificationDependencies,
): () => void {
  const notifications = new Map<string, () => void>();

  function senderWindow(event: IpcMainInvokeEvent): BrowserWindow | null {
    const window = getWindow();
    return window && !window.isDestroyed()
      && event.sender === window.webContents && event.senderFrame === window.webContents.mainFrame ? window : null;
  }

  ipc.handle("show-session-attention-notification", (event, id: unknown) => {
    const window = senderWindow(event);
    if (!window || window.isFocused() || !enabled() || !isSupported()
        || typeof id !== "string" || !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(id) || notifications.has(id)) return false;

    return new Promise<boolean>((resolve) => {
      const notification = create({
        title: "A session needs you",
        body: "Open OrkWorks to respond.",
        silent: true,
      });
      const clear = (clicked: boolean) => {
        if (notifications.get(id) !== close) return;
        notifications.delete(id);
        resolve(clicked);
      };
      const close = () => {
        clear(false);
        notification.close();
      };
      notifications.set(id, close);
      notification.once("close", () => clear(false));
      notification.once("failed", () => clear(false));
      notification.once("click", () => {
        if (notifications.get(id) !== close) return;
        if (window.isDestroyed() || !enabled()) {
          close();
          return;
        }
        if (window.isMinimized()) window.restore();
        window.show();
        window.focus();
        clear(true);
        notification.close();
      });
      try {
        notification.show();
      } catch {
        clear(false);
      }
    });
  });

  ipc.handle("close-session-attention-notification", (event, id: unknown) => {
    if (senderWindow(event) && typeof id === "string") notifications.get(id)?.();
  });

  return () => {
    for (const close of notifications.values()) close();
  };
}
