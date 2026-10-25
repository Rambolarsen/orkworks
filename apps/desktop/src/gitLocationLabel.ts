export function gitLocationLabel(source?: "agent" | "harness" | "process" | "launch"): string {
  switch (source) {
    case "agent": return "Reported task worktree";
    case "harness": return "Harness-reported directory";
    case "process": return "Process directory";
    default: return "Launch directory; task worktree not reported";
  }
}
