import { effectiveLifecyclePhase, type SessionInfo } from "../api.ts";

declare const __sessionIdBrand: unique symbol;
export type SessionId = string & { readonly [__sessionIdBrand]: true };

export const SessionStatus = {
  Creating: "creating",
  Running: "running",
  Killed: "killed",
  Ended: "ended",
  Error: "error",
} as const;
export type SessionStatus = (typeof SessionStatus)[keyof typeof SessionStatus];

export const MemoryState = {
  Live: "live",
  Remembered: "remembered",
  Resumable: "resumable",
  Unsupported: "unsupported",
} as const;
export type MemoryState = (typeof MemoryState)[keyof typeof MemoryState];

export const AttentionState = {
  WaitingForInput: "waiting_for_input",
  Blocked: "blocked",
  Failed: "failed",
  Done: "done",
  Stale: "stale",
  Working: "working",
  Idle: "idle",
} as const;
export type AttentionState = (typeof AttentionState)[keyof typeof AttentionState];

export const WorkPhaseValue = {
  Ideation: "ideation",
  Implementation: "implementation",
  Review: "review",
  Debugging: "debugging",
  Unknown: "unknown",
} as const;
export type WorkPhaseValue = (typeof WorkPhaseValue)[keyof typeof WorkPhaseValue];

export const LifecyclePhaseValue = {
  Creating: "creating",
  Active: "active",
  Ending: "ending",
  Ended: "ended",
} as const;
export type LifecyclePhaseValue = (typeof LifecyclePhaseValue)[keyof typeof LifecyclePhaseValue];

export interface Session {
  id: SessionId;
  label: string;
  workspacePath: string;
  status: SessionStatus;
  memoryState: MemoryState;
  attentionState: AttentionState;
  workPhase: WorkPhaseValue;
  lifecyclePhase: LifecyclePhaseValue;
  created: Date;
  killed?: Date;
  lastActive?: Date;
  cwd: string;
  harnessName?: string;
  providerId?: string;
  taskDescription?: string;
  model?: string;
  repoRoot?: string;
  branch?: string;
  dirty?: boolean;
  changedFiles?: number;
  isWorktree?: boolean;
  observedStatus?: string;
  finalObservedStatus?: string | null;
  metadataSource?: string;
  metadataConfidence?: number;
  summary?: string;
  nextAction?: string;
  needsUserInput?: boolean;
  detectedQuestion?: string;
  suggestedOptions?: string[];
  blockerDescription?: string;
  failedCommand?: string;
  failedTest?: string;
  capacityHints?: string[];
  capacityCheckPending?: boolean;
  peonLastInference?: string;
  conflictWarning?: string;
  recommendation?: string;
  provider?: string;
  providerModel?: string;
  providerState?: string;
  resumeStrategy?: string;
  resume?: Record<string, unknown>;
  resumedFrom?: string;
}

export const ATTENTION_PRIORITY: Record<string, number> = {
  [AttentionState.WaitingForInput]: 0,
  [AttentionState.Blocked]: 1,
  [AttentionState.Failed]: 2,
  [AttentionState.Done]: 3,
  [AttentionState.Stale]: 4,
  [AttentionState.Working]: 5,
  [AttentionState.Idle]: 6,
  creating: 7,
  running: 8,
  ended: 9,
  killed: 10,
  error: 11,
};

export function needsAttention(session: Session): boolean {
  const state = sessionAttentionStatus(session);
  return state === AttentionState.Blocked
    || state === AttentionState.Failed
    || state === AttentionState.WaitingForInput;
}

export function sessionAttentionStatus(session: Session): string {
  if (session.capacityCheckPending) {
    return "checking_capacity";
  }
  if (effectiveLifecyclePhase(session.status, session.lifecyclePhase) === "active") {
    return session.observedStatus ?? session.status;
  }
  return session.finalObservedStatus ?? session.status;
}

export function sortSessions(sessions: Session[]): Session[] {
  return [...sessions].sort((a, b) => {
    const la = a.memoryState === MemoryState.Live ? 0 : 1;
    const lb = b.memoryState === MemoryState.Live ? 0 : 1;
    if (la !== lb) return la - lb;
    const pa = ATTENTION_PRIORITY[sessionAttentionStatus(a)] ?? 99;
    const pb = ATTENTION_PRIORITY[sessionAttentionStatus(b)] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.label.localeCompare(b.label);
  });
}

export function fromApiDto(dto: SessionInfo): Session {
  const lifecyclePhase = effectiveLifecyclePhase(dto.status, dto.lifecyclePhase);
  const session: Session = {
    id: dto.id as SessionId,
    label: dto.label,
    workspacePath: dto.cwd,
    status: dto.status as SessionStatus,
    memoryState: dto.memoryState as MemoryState,
    attentionState: AttentionState.Working,
    workPhase: (dto.workPhase ?? WorkPhaseValue.Unknown) as WorkPhaseValue,
    lifecyclePhase: lifecyclePhase as LifecyclePhaseValue,
    created: new Date(dto.created_at),
    lastActive: dto.peonLastInference ? new Date(dto.peonLastInference) : undefined,
    cwd: dto.cwd,
    harnessName: dto.harness,
    providerId: dto.provider,
    taskDescription: undefined,
    model: dto.model,
    repoRoot: dto.repoRoot,
    branch: dto.branch,
    dirty: dto.dirty,
    changedFiles: dto.changedFiles,
    isWorktree: dto.isWorktree,
    observedStatus: dto.observedStatus,
    finalObservedStatus: dto.finalObservedStatus,
    metadataSource: dto.metadataSource,
    metadataConfidence: dto.metadataConfidence,
    summary: dto.summary,
    nextAction: dto.nextAction,
    needsUserInput: dto.needsUserInput,
    detectedQuestion: dto.detectedQuestion,
    suggestedOptions: dto.suggestedOptions,
    blockerDescription: dto.blockerDescription,
    failedCommand: dto.failedCommand,
    failedTest: dto.failedTest,
    capacityHints: dto.capacityHints,
    peonLastInference: dto.peonLastInference,
    conflictWarning: dto.conflictWarning,
    recommendation: dto.recommendation,
    provider: dto.provider,
    providerModel: dto.providerModel,
    providerState: dto.providerState,
    resumeStrategy: dto.resumeStrategy,
    resume: dto.resume as Record<string, unknown> | undefined,
    resumedFrom: dto.resumedFrom,
  };
  session.attentionState = sessionAttentionStatus(session) as AttentionState;
  return session;
}
