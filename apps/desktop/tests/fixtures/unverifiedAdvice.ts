import type { WorkflowRecommendation } from '../../src/api.ts';

export function unverifiedRecommendation(): WorkflowRecommendation {
  return {
    id: 'rec-1', workspaceId: '/repo', chainId: 'chain-rec-1', chainDepth: 0,
    type: 'improve_workflow', status: 'proposed', priority: 'medium',
    title: 'A proposed validation', summary: 'Tests passed.', reason: ['A bounded hypothesis.'],
    protocolVersion: 3, verificationStatus: 'unverified',
    assessment: {
      id: 'improvement-11111111-1111-4111-8111-111111111111', inputFingerprint: 'b'.repeat(64),
      contextLevel: 'source_code', excludedPaths: [], accessRedacted: false,
      improvementReference: {version: 3, intent: 'implementation'},
    },
    evidence: [], repositoryEvidence: [{path: 'AGENTS.md', sha256: 'a'.repeat(64), excerpt: 'Verification remains unverified.', observedAt: '2026-11-17T10:00:00Z'}],
    knowledgeEvidence: [{pageId: 'playbooks/taskmaster-improvement.md', title: 'Improvement', status: 'stable', bundleVersion: 'brain-4', sha256: 'c'.repeat(64), excerpt: 'Bounded advice.'}],
    sourceSessionIds: [], targetSessionId: null, suggestedHarnessId: null, suggestedModel: null,
    suggestedWorkingDirectory: null, suggestedPrompt: null, confidence: 'medium', requiresApproval: false,
    dedupeKey: `proactive:v1:instructions:${'a'.repeat(64)}`, expiresAt: null,
    workflowImprovement: {proposedImprovement: 'Review the supplied mechanism.', targetSurface: 'instructions', observationIds: [], recurrenceCount: 0,
      affectedSessionIds: [], impact: 'medium', expectedBenefit: 'A possible improvement.', supersedesRecommendationId: null, dismissalWatermark: null},
    createdAt: '2026-11-17T10:00:00Z', updatedAt: '2026-11-17T10:00:00Z',
    rollupMemberIds: [], rollupMemberDedupeKeys: [], rollupGeneration: null, rolledUpBy: null,
    completionPacket: null, proposedChange: null,
  };
}
