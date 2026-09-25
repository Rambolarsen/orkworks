import type { SessionInfo, WorkflowRecommendation } from "./api";

export function handleAcceptedFixWithAi(
  acceptedRecommendation: Pick<WorkflowRecommendation, "targetSessionId">,
  sessions: readonly Pick<SessionInfo, "id" | "label">[],
  selectSession: (sessionId: string) => void,
  showInfo: (message: string) => void,
): void {
  const targetSessionId = acceptedRecommendation.targetSessionId;
  if (!targetSessionId) return;

  selectSession(targetSessionId);
  const targetSession = sessions.find((session) => session.id === targetSessionId);
  showInfo(`Fix sent to ${targetSession?.label ?? targetSessionId}.`);
}
