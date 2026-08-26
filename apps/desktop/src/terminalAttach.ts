export async function attachTerminalAfterBackendReady(
  getBackendUrl: () => Promise<string>,
  isCancelled: () => boolean,
  attach: (baseUrl: string) => void,
  onUnavailable: () => void,
): Promise<void> {
  try {
    const baseUrl = await getBackendUrl();
    if (!isCancelled()) attach(baseUrl);
  } catch {
    if (!isCancelled()) onUnavailable();
  }
}
