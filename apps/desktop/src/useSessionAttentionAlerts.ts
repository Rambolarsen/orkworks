import { useLayoutEffect, useRef } from "react";
import type { SessionInfo } from "./api";
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

export function useSessionAttentionAlerts(options: Options): void {
  const snapshot = useRef<ReturnType<typeof observeSessionAttention>["snapshot"]>(null);
  const active = useRef(new Map<string, ActiveAlert>());
  const deferred = useRef(new Map<string, SessionInfo>());
  const openSession = useRef(options.openSession);
  const { scope, sessions, visibleSessionId, desktopEnabled, dialogOpen } = options;

  useLayoutEffect(() => {
    openSession.current = options.openSession;
  }, [options.openSession]);

  useLayoutEffect(() => {
    const alerts = active.current;
    const deferredWaits = deferred.current;
    return () => {
      for (const alert of alerts.values()) alert.close();
      alerts.clear();
      deferredWaits.clear();
      snapshot.current = null;
    };
  }, []);

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
