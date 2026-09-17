export type BackendStatus = "picker" | "connecting…" | "connected" | "unreachable" | "exhausted";

export function shouldEnableSessionPolling(
  backendStatus: BackendStatus,
  hasWorkspace: boolean,
  isSwitchingWorkspace: boolean,
): boolean {
  return backendStatus === "connected" && hasWorkspace && !isSwitchingWorkspace;
}
