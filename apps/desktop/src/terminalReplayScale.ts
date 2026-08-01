export function computeReplayScale(
  natural: { width: number; height: number },
  available: { width: number; height: number },
): number {
  if (natural.width <= 0 || natural.height <= 0) return 1;
  return Math.min(1, available.width / natural.width, available.height / natural.height);
}

/**
 * Returns the content-box size of an element from its padding-inclusive
 * clientWidth/clientHeight and its per-side CSS padding. xterm lives inside
 * `.terminal-container`'s padding, so scaling against padding-inclusive
 * clientWidth/clientHeight overshoots and `overflow: hidden` clips the
 * right/bottom edges. Clamped at 0 so a degenerate or oversized padding
 * degrades to an empty box rather than a negative one.
 */
export function availableContentBox(
  clientSize: { width: number; height: number },
  padding: { top: number; right: number; bottom: number; left: number },
): { width: number; height: number } {
  return {
    width: Math.max(0, clientSize.width - padding.left - padding.right),
    height: Math.max(0, clientSize.height - padding.top - padding.bottom),
  };
}
