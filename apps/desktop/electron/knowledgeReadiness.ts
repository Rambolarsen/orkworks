// Code-owned rubric vocabulary; renderer/sidecar own their independent contracts.
export const READINESS_CONCEPT_IDS = [
  'context-discovery', 'progressive-disclosure', 'instruction-scoping',
  'architecture-awareness', 'domain-knowledge', 'environment-reproducibility',
  'tool-accessibility', 'task-decomposition', 'verification', 'observability',
  'change-safety', 'feedback-loops', 'durable-memory', 'human-oversight',
] as const;
export type ReadinessRubric = {
  readonly id: 'brain-agent-readiness';
  readonly revision: number;
  readonly concepts: readonly {
    readonly conceptId: string;
    readonly pageId: string;
    readonly pageSha256: string;
    readonly levels: readonly string[];
  }[];
};
function exact(value: unknown, keys: string[]): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      Object.keys(value).sort().join('\0') !== keys.sort().join('\0')) {
    throw new Error('Invalid readiness rubric fields');
  }
  return value as Record<string, unknown>;
}
export function parseReadinessRubric(value: unknown, pages: readonly {id: string; sha256: string}[]): ReadinessRubric {
  const source = exact(value, ['id', 'revision', 'concepts']);
  if (source.id !== 'brain-agent-readiness' || !Number.isSafeInteger(source.revision) || (source.revision as number) < 1 ||
      !Array.isArray(source.concepts) || source.concepts.length !== 14 || Buffer.byteLength(JSON.stringify(source), 'utf8') > 40 * 1024) {
    throw new Error('Invalid readiness rubric identity or bounds');
  }
  const concepts = source.concepts.map((item, index) => {
    const entry = exact(item, ['conceptId', 'pageId', 'pageSha256', 'levels']);
    if (entry.conceptId !== READINESS_CONCEPT_IDS[index] ||
        !pages.some(page => page.id === entry.pageId && page.sha256 === entry.pageSha256) ||
        !Array.isArray(entry.levels) || entry.levels.length !== 5 ||
        entry.levels.some(level => typeof level !== 'string' || !level.trim() || Buffer.byteLength(level, 'utf8') > 500 || /[\x00-\x1f\x7f]/.test(level))) {
      throw new Error('Invalid readiness concept mapping or levels');
    }
    return { conceptId: entry.conceptId as string, pageId: entry.pageId as string, pageSha256: entry.pageSha256 as string, levels: entry.levels as string[] };
  });
  return {id: 'brain-agent-readiness', revision: source.revision as number, concepts};
}
