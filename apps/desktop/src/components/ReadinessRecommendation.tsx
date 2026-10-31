import type { WorkflowRecommendation } from '../api';
import { conceptIds, conceptLabel, type ReadinessSnapshot } from '../readiness';

export default function ReadinessRecommendation({ recommendation, snapshots, onOpen }: {
  recommendation: WorkflowRecommendation;
  snapshots: ReadinessSnapshot[];
  onOpen: (id: string) => void;
}) {
  const reference = recommendation.assessment?.readinessReference;
  if (!reference || recommendation.assessment?.accessRedacted
    || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(reference.snapshotId)
    || !/^[0-9a-f]{64}$/.test(reference.rubricDigest)
    || !Array.isArray(reference.conceptIds) || reference.conceptIds.length < 1 || reference.conceptIds.length > 3
    || new Set(reference.conceptIds).size !== reference.conceptIds.length
    || reference.conceptIds.some(id => !(conceptIds as readonly string[]).includes(id))) return null;
  const source = snapshots.find(snapshot => snapshot.id === reference.snapshotId && !snapshot.redacted
    && snapshot.data?.workspacePath === recommendation.workspaceId
    && snapshot.data.rubricDigest === reference.rubricDigest);
  return <div className="recommendation-readiness">
    <p><strong>Readiness capability:</strong> {reference.conceptIds.map(conceptLabel).join(', ')}</p>
    {source ? <button type="button" onClick={() => onOpen(source.id)}>View saved assessment · {new Date(source.completedAt).toLocaleString()}</button>
      : <p>Saved assessment unavailable.</p>}
  </div>;
}
