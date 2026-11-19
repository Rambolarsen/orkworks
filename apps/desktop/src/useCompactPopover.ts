import { useLayoutEffect, type RefObject } from "react";

/** Keep a compact toolbar menu beside its invoker, within the viewport. */
export function useCompactPopover(
  enabled: boolean,
  anchorRef: RefObject<HTMLElement | null>,
  panelRef: RefObject<HTMLElement | null>,
  align: "start" | "end",
): void {
  useLayoutEffect(() => {
    const anchor = anchorRef.current;
    const panel = panelRef.current;
    if (!enabled || !anchor || !panel) return;
    const originalStyle = panel.style.cssText;
    const update = () => {
      if (!anchor.isConnected || !panel.isConnected) return;
      panel.style.position = "fixed";
      panel.style.right = "auto";
      panel.style.transform = "none";
      panel.style.overflowY = "auto";
      if (!panel.offsetWidth || !anchor.getClientRects().length) return;
      const bounds = anchor.getBoundingClientRect();
      const width = panel.getBoundingClientRect().width;
      const height = panel.scrollHeight + panel.offsetHeight - panel.clientHeight;
      const margin = 12;
      const gap = 6;
      const below = Math.max(0, window.innerHeight - bounds.bottom - gap - margin);
      const above = Math.max(0, bounds.top - gap - margin);
      const placeBelow = height <= below || below >= above;
      const available = placeBelow ? below : above;
      const left = align === "start" ? bounds.left : bounds.right - width;
      panel.style.left = `${Math.max(margin, Math.min(left, window.innerWidth - width - margin))}px`;
      panel.style.top = `${placeBelow ? bounds.bottom + gap : bounds.top - gap - Math.min(height, available)}px`;
      panel.style.maxHeight = `${available}px`;
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(panel);
    observer.observe(anchor);
    const toolbar = anchor.closest(".shell-toolbar");
    if (toolbar) observer.observe(toolbar);
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
      panel.style.cssText = originalStyle;
    };
  }, [enabled, anchorRef, panelRef, align]);
}
