export function computeReplayScale(
  natural: { width: number; height: number },
  available: { width: number; height: number },
): number {
  if (natural.width <= 0 || natural.height <= 0) return 1;
  return Math.min(1, available.width / natural.width, available.height / natural.height);
}
