export type HarnessLaunchId = "claude-code";

export function terminalLaunchInput(harness: HarnessLaunchId): string {
  switch (harness) {
    case "claude-code":
      return "claude\n";
  }
}
