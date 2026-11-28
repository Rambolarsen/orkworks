import { useLayoutEffect, useRef } from "react";
import type { SessionInfo } from "./api";
import { autoArchiveWarningKey, collectNewAutoArchiveWarnings, type AutoArchiveNotificationState } from "./autoArchiveWarning";
import { dismissToast, pushToast } from "./feedback";
import { observeSessionAttention, sessionAlertDelivery } from "./sessionAttentionAlerts";

interface Options {
  scope: string | null;
  sessions: readonly SessionInfo[];
  visibleSessionId: string | null;
  desktopEnabled: boolean;
  dialogOpen: boolean;
  openSession: (id: string) => void;
}

interface ActiveAlert {
  scope: string;
  delivery: "toast" | "desktop";
  close: () => void;
}

interface ActiveAutoArchiveAlert {
  scope: string;
  key: string;
  close: () => void;
}

export function useSessionAttentionAlerts(options: Options): void {
  const snapshot = useRef<ReturnType<typeof observeSessionAttention>["snapshot"]>(null);
  const autoArchiveState = useRef<AutoArchiveNotificationState | null>(null);
  const activeAutoArchive = useRef(new Map<string, ActiveAutoArchiveAlert>());
  const active = useRef(new Map<string, ActiveAlert>());
  const deferred = useRef(new Map<string, SessionInfo>());
  const openSession = useRef(options.openSession);
  const dialogOpenRef = useRef(options.dialogOpen);
  const { scope, sessions, visibleSessionId, desktopEnabled, dialogOpen } = options;

  useLayoutEffect(() => {
    openSession.current = options.openSession;
  }, [options.openSession]);

  useLayoutEffect(() => {
    dialogOpenRef.current = dialogOpen;
  }, [dialogOpen]);

  useLayoutEffect(() => {
    const alerts = active.current;
    const autoArchiveAlerts = activeAutoArchive.current;
    const deferredWaits = deferred.current;
    return () => {
      for (const alert of alerts.values()) alert.close();
      alerts.clear();
      for (const alert of autoArchiveAlerts.values()) alert.close();
      autoArchiveAlerts.clear();
      deferredWaits.clear();
      snapshot.current = null;
      autoArchiveState.current = null;
    };
  }, []);

  useLayoutEffect(() => {
    const currentWarningKeys = new Set<string>();
    if (scope !== null && desktopEnabled) {
      for (const session of sessions) {
        if (session.sessionState !== "active" || session.favorited === true) continue;
        const key = autoArchiveWarningKey(session);
        if (key) currentWarningKeys.add(key);
      }
    }
    for (const [key, alert] of activeAutoArchive.current) {
      if (desktopEnabled && scope !== null && alert.scope === scope && currentWarningKeys.has(key)) continue;
      activeAutoArchive.current.delete(key);
      alert.close();
    }
    if (!desktopEnabled || scope === null) {
      if (scope === null) autoArchiveState.current = null;
      return;
    }
    const { state, toNotify } = collectNewAutoArchiveWarnings(autoArchiveState.current, scope, sessions);
    autoArchiveState.current = state;
    for (const session of toNotify) {
      const key = autoArchiveWarningKey(session);
      if (!key) continue;
      const notificationId = crypto.randomUUID();
      const alert: ActiveAutoArchiveAlert = {
        scope,
        key,
        close: () => {
          void window.orkworks.closeSessionAttentionNotification(notificationId).catch(() => {});
        },
      };
      activeAutoArchive.current.set(key, alert);
      void window.orkworks
        .showSessionAttentionNotification(notificationId, "auto_archive_warning")
        .then((clicked) => {
          const stillCurrent = activeAutoArchive.current.get(key) === alert;
          if (stillCurrent) activeAutoArchive.current.delete(key);
          if (clicked && stillCurrent && !dialogOpenRef.current) {
            openSession.current(session.id);
          }
        })
        .catch(() => {
          if (activeAutoArchive.current.get(key) === alert) activeAutoArchive.current.delete(key);
        });
    }
  }, [scope, sessions, desktopEnabled]);

  useLayoutEffect(() => {
    if (snapshot.current?.scope !== scope) deferred.current.clear();
    const next = observeSessionAttention(snapshot.current, scope, sessions);
    snapshot.current = next.snapshot;
    const focused = document.visibilityState === "visible" && document.hasFocus();
    for (const [id, alert] of active.current) {
      if (!dialogOpen && alert.scope === scope && next.snapshot?.waitingIds.has(id)
          && (alert.delivery !== "desktop" || desktopEnabled)
          && (!focused || id !== visibleSessionId)) continue;
      active.current.delete(id);
      alert.close();
    }

    if (scope === null) return;
    for (const id of deferred.current.keys()) {
      if (!next.snapshot?.waitingIds.has(id)) deferred.current.delete(id);
    }
    // Retain new waits until a dialog can no longer block navigation.
    for (const session of next.newWaits) deferred.current.set(session.id, session);
    if (dialogOpen) return;

    for (const session of deferred.current.values()) {
      const delivery = sessionAlertDelivery(session.id, visibleSessionId, focused, desktopEnabled);
      if (!delivery) continue;
      const alert: ActiveAlert = { scope, delivery, close: () => {} };
      const open = () => {
        if (active.current.get(session.id) !== alert) return;
        active.current.delete(session.id);
        openSession.current(session.id);
      };
      active.current.set(session.id, alert);
      if (delivery === "toast") {
        const id = pushToast("warn", `${session.label} needs you`, 8000, { label: "Open session", run: open });
        alert.close = () => dismissToast(id);
      } else {
        const id = crypto.randomUUID();
        alert.close = () => { void window.orkworks.closeSessionAttentionNotification(id).catch(() => {}); };
        void window.orkworks.showSessionAttentionNotification(id).then((clicked) => {
          if (clicked) open();
          else if (active.current.get(session.id) === alert) active.current.delete(session.id);
        }).catch(() => {
          if (active.current.get(session.id) === alert) active.current.delete(session.id);
        });
      }
    }
    deferred.current.clear();
  }, [scope, sessions, visibleSessionId, desktopEnabled, dialogOpen]);
}
