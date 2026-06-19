export type ToastTone = "info" | "warn" | "error";

export interface Toast {
  id: string;
  tone: ToastTone;
  message: string;
}

type Listener = (toasts: readonly Toast[]) => void;

const state: Toast[] = [];
const listeners = new Set<Listener>();
let counter = 0;

function emit(): void {
  const snapshot = [...state];
  for (const l of listeners) l(snapshot);
}

export function pushToast(tone: ToastTone, message: string, timeoutMs = 4000): string {
  const id = `t${++counter}`;
  state.push({ id, tone, message });
  emit();
  if (timeoutMs > 0) {
    setTimeout(() => dismissToast(id), timeoutMs);
  }
  return id;
}

export function dismissToast(id: string): void {
  const idx = state.findIndex((t) => t.id === id);
  if (idx >= 0) {
    state.splice(idx, 1);
    emit();
  }
}

export function subscribeToasts(l: Listener): () => void {
  listeners.add(l);
  l([...state]);
  return () => {
    listeners.delete(l);
  };
}
