import { useLayoutEffect, useRef, type ReactNode } from "react";

interface ModalBackdropProps {
  className: string;
  labelledBy: string;
  describedBy?: string;
  onCancel: () => void;
  children: ReactNode;
}

const fallbackSelectors = [
  "[data-shell-settings]",
  ".shell-navigation-menu > summary",
  "[data-shell-create]",
  ".titlebar-open-button",
  "[data-shell-terminal-focus]",
];

export default function ModalBackdrop({ className, labelledBy, describedBy, onCancel, children }: ModalBackdropProps) {
  const ref = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    dialog.showModal();
    return () => {
      dialog.close();
      // Wait for layout switches and a possible handoff to another dialog.
      requestAnimationFrame(() => {
        if (document.activeElement !== document.body || document.querySelector("dialog[open]")) return;
        for (const selector of fallbackSelectors) {
          const target = [...document.querySelectorAll<HTMLElement>(selector)]
            .find(element => element.getClientRects().length > 0 && !element.matches(":disabled"));
          if (!target) continue;
          target.focus({ preventScroll: true });
          if (document.activeElement === target) return;
        }
      });
    };
  }, []);

  return <dialog ref={ref} className={`modal-backdrop ${className}`}
    aria-labelledby={labelledBy} aria-describedby={describedBy}
    onCancel={event => { event.preventDefault(); onCancel(); }}>
    {children}
  </dialog>;
}
