export type SessionTitleRequest = {
  sessionId: string;
  workspaceIdentity: string;
} & ({ action: "read" | "automatic"; title?: never } | { action: "set"; title: string });
export interface SessionTitleResult { title: string; automatic: boolean }
