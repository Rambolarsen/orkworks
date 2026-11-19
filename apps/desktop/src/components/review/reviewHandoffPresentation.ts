/** Local presentation records only: no wire format or approval authority. */
export type ReviewHandoffArtifact =
  | Readonly<{ state: "available"; path: string; anchor: string; version: string; pinned: boolean }>
  | Readonly<{ state: "missing" }>
  | Readonly<{ state: "unavailable"; reason: string }>;
export type ReviewHandoffSnapshot = Readonly<{
  scope: string; proposalId: string; revision: string;
  source: Readonly<{ id: string; runtime: string; title: string; tool: string }>;
  reviewer: Readonly<{ label: string; role: string; tool: string; model: string }>;
  sourceLocation: Readonly<{ worktree: string | null; branch: string | null }>;
  reviewerLocation: Readonly<{ worktree: string | null; branch: string | null }>;
  artifact: ReviewHandoffArtifact; instruction: string; unavailableReason: string;
}>;
export type ReviewHandoffState = Readonly<{
  snapshot: ReviewHandoffSnapshot | null; staleReason: string | null;
  epoch: number; request: number | null; refreshCount: number;
}>;
export type ReviewHandoffEvent =
  | { type: "open"; snapshot: ReviewHandoffSnapshot }
  | { type: "close" | "reset" }
  | { type: "changed"; reason: string }
  | { type: "refresh-started" }
  | { type: "refresh-completed"; request: number; snapshot: ReviewHandoffSnapshot }
  | { type: "refresh-failed"; request: number; reason: string };
export const initialReviewHandoffState: ReviewHandoffState = {
  snapshot: null, staleReason: null, epoch: 0, request: null, refreshCount: 0,
};
export function reviewHandoffReducer(state: ReviewHandoffState, event: ReviewHandoffEvent): ReviewHandoffState {
  switch (event.type) {
    case "open": return { ...initialReviewHandoffState, snapshot: event.snapshot, epoch: state.epoch + 1 };
    case "close": case "reset": return { ...initialReviewHandoffState, epoch: state.epoch + 1 };
    case "changed": return state.snapshot ? { ...state, staleReason: event.reason, request: null, epoch: state.epoch + 1 } : state;
    case "refresh-started": return state.snapshot ? { ...state, epoch: state.epoch + 1, request: state.epoch + 1 } : state;
    case "refresh-completed": {
      if (!state.snapshot || state.request !== event.request) return state;
      if (event.snapshot.scope !== state.snapshot.scope || event.snapshot.proposalId !== state.snapshot.proposalId) {
        return { ...state, request: null, epoch: state.epoch + 1, staleReason: "Review context changed; refresh again." };
      }
      return { ...state, snapshot: event.snapshot, staleReason: null, request: null, refreshCount: state.refreshCount + 1 };
    }
    case "refresh-failed": return state.snapshot && state.request === event.request
      ? { ...state, request: null, epoch: state.epoch + 1, staleReason: event.reason } : state;
  }
}
