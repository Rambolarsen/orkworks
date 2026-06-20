import { useEffect, useState } from "react";
import { dismissToast, subscribeToasts, type Toast } from "../feedback";

export default function ToastRack() {
  const [toasts, setToasts] = useState<readonly Toast[]>([]);

  useEffect(() => subscribeToasts(setToasts), []);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-rack" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className="toast" data-tone={t.tone}>
          <span className="toast-message">{t.message}</span>
          <button
            type="button"
            className="toast-dismiss"
            aria-label="Dismiss"
            onClick={() => dismissToast(t.id)}
          >
            &times;
          </button>
        </div>
      ))}
    </div>
  );
}
