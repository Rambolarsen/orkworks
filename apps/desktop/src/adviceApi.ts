import { parseAdviceResponse } from './advice.ts';
import type { AcceptRecommendationOptions, ManualTaskmasterAnalysisResponse, RecommendationListResponse, TaskmasterAssessmentRequestResponse, TaskmasterAuditResponse, WorkflowRecommendation } from './api.ts';
import type { ImprovementAdmission, ImprovementCancellation } from './improvement.ts';
import type { ReadinessAdmission } from './readiness.ts';
import type { TaskmasterRunStatus } from './taskmasterSettings.ts';

function method<T>(value: T): T {
  if (typeof value !== 'function') throw Error('Advice protocol upgrade required');
  return value;
}
function inWorkspace(item: Record<string, unknown>, workspacePath: string): void {
  if (item.workspaceId !== workspacePath) throw Error('Workspace changed; reload Recommendations');
}
function admission(value: unknown): ImprovementAdmission {
  const item = parseAdviceResponse(value);
  if (!['scheduled', 'already_running', 'unavailable'].includes(item.outcome as string)
    || !(item.runId === null || typeof item.runId === 'string' && /^[A-Za-z0-9-]{1,128}$/.test(item.runId))
    || !(item.reasonCode === null || typeof item.reasonCode === 'string' && item.reasonCode.length <= 128)) throw Error('Invalid advice admission');
  return item as unknown as ImprovementAdmission;
}

export async function getAdviceRecommendations(baseUrl: string, workspacePath: string): Promise<RecommendationListResponse> {
  const item = parseAdviceResponse(await method(window.orkworks.getAdviceRecommendations)(workspacePath, baseUrl));
  if (!Array.isArray(item.recommendations) || !Array.isArray(item.diagnostics)) throw Error('Invalid advice list');
  item.recommendations.forEach(value => inWorkspace(value, workspacePath));
  return item as unknown as RecommendationListResponse;
}
export async function getAdviceRecommendation(baseUrl: string, id: string, workspacePath: string): Promise<WorkflowRecommendation> {
  const item = parseAdviceResponse(await method(window.orkworks.getAdviceRecommendation)(workspacePath, baseUrl, id));
  inWorkspace(item, workspacePath);
  if (item.id !== id) throw Error('Invalid recommendation identity');
  return item as unknown as WorkflowRecommendation;
}
export async function runAdviceAudit(baseUrl: string, workspacePath: string): Promise<TaskmasterAuditResponse> {
  const item = parseAdviceResponse(await method(window.orkworks.runAdviceAudit)(workspacePath, baseUrl));
  if (item.recommendation) inWorkspace(item.recommendation as Record<string, unknown>, workspacePath);
  return item as unknown as TaskmasterAuditResponse;
}
export async function acceptAdviceRecommendation(workspacePath: string, id: string, options: AcceptRecommendationOptions): Promise<Record<string, unknown>> {
  return parseAdviceResponse(await method(window.orkworks.acceptAdviceRecommendation)(workspacePath, id, options));
}
export async function dismissAdviceRecommendation(workspacePath: string, id: string, reason?: string): Promise<void> {
  parseAdviceResponse(await method(window.orkworks.dismissAdviceRecommendation)(workspacePath, id, reason));
}
export async function getAdviceReadinessStatus(workspacePath: string): Promise<Record<string, unknown>> {
  return parseAdviceResponse(await method(window.orkworks.getAdviceReadinessStatus)(workspacePath));
}
export async function getAdviceImprovementStatus(workspacePath: string): Promise<Record<string, unknown>> {
  return parseAdviceResponse(await method(window.orkworks.getAdviceImprovementStatus)(workspacePath));
}
export async function requestAdviceImprovement(workspacePath: string): Promise<ImprovementAdmission> {
  return admission(await method(window.orkworks.requestAdviceImprovement)(workspacePath));
}
export async function requestAdviceReadinessAssessment(workspacePath: string): Promise<ReadinessAdmission> {
  return admission(await method(window.orkworks.requestAdviceReadinessAssessment)(workspacePath));
}
export async function requestAdviceReadinessSuggestion(workspacePath: string): Promise<ReadinessAdmission> {
  return admission(await method(window.orkworks.requestAdviceReadinessSuggestion)(workspacePath));
}
export async function cancelAdviceImprovement(workspacePath: string, runId: string): Promise<ImprovementCancellation> {
  const item = parseAdviceResponse(await method(window.orkworks.cancelAdviceImprovement)(workspacePath, runId));
  if (!['cancel_requested', 'not_running', 'unavailable'].includes(item.outcome as string)
    || !(item.runId === null || item.runId === runId)
    || !(item.reasonCode === null || typeof item.reasonCode === 'string' && item.reasonCode.length <= 128)) throw Error('Invalid advice cancellation');
  return item as unknown as ImprovementCancellation;
}
export async function getAdviceRunStatus(workspacePath: string): Promise<TaskmasterRunStatus> {
  return parseAdviceResponse(await method(window.orkworks.getAdviceRunStatus)(workspacePath)) as unknown as TaskmasterRunStatus;
}
export async function requestAdviceAnalysis(workspacePath: string): Promise<ManualTaskmasterAnalysisResponse> {
  return parseAdviceResponse(await method(window.orkworks.requestAdviceAnalysis)(workspacePath)) as unknown as ManualTaskmasterAnalysisResponse;
}
export async function requestAdviceAssessment(workspacePath: string): Promise<TaskmasterAssessmentRequestResponse> {
  return parseAdviceResponse(await method(window.orkworks.requestAdviceAssessment)(workspacePath)) as unknown as TaskmasterAssessmentRequestResponse;
}
