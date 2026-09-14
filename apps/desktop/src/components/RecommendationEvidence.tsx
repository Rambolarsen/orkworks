import { useEffect, useRef, useState } from "react";
import { getTaskmasterRecommendation, type WorkflowRecommendation } from "../api.ts";
import { sortedEvidence } from "../taskmaster.ts";

interface EvidenceProps {
  recommendation: WorkflowRecommendation;
  onSelectSession?: (id: string) => void;
}

function EvidenceRows({ recommendation, onSelectSession }: EvidenceProps) {
  return <>
    {recommendation.repositoryEvidence?.map((item) => <div className="recommendation-evidence-row" key={`${item.path}:${item.sha256}`}>
      <strong>{item.path}</strong><span>Repository snapshot · {item.observedAt}</span><p>{item.excerpt}</p>
    </div>)}
    {recommendation.knowledgeEvidence?.map((item) => <div className="recommendation-evidence-row" key={`${item.pageId}:${item.sha256}`}>
      <strong>{item.title}</strong><span>Knowledge {item.bundleVersion} · {item.status}</span><p>{item.excerpt}</p>
      <span>{item.pageId}</span>
    </div>)}
    {sortedEvidence(recommendation.evidence).map((item) => (
      <div className="recommendation-evidence-row" key={item.observationId}>
        <strong>{item.description}</strong>
        <span>{item.source} · {item.observedAt}</span>
        {item.problemArea && <span>Problem area · {item.problemArea}</span>}
        <p>{item.evidence}</p>
        <button type="button" onClick={() => onSelectSession?.(item.sessionId)}>
          Open session {item.sessionId.slice(0, 8)}
        </button>
      </div>
    ))}
  </>;
}

interface MemberDetails {
  key: string;
  members: Map<string, WorkflowRecommendation>;
  loading: boolean;
  failed: boolean;
}

export default function RecommendationEvidence({ recommendation, onSelectSession }: EvidenceProps) {
  const [expanded, setExpanded] = useState(false);
  const [retry, setRetry] = useState(0);
  const key = JSON.stringify([recommendation.id, recommendation.updatedAt, recommendation.rollupMemberIds]);
  const cache = useRef<MemberDetails>({ key: "", members: new Map(), loading: false, failed: false });
  const [details, setDetails] = useState(cache.current);

  useEffect(() => {
    if (cache.current.key !== key) {
      cache.current = { key, members: new Map(), loading: false, failed: false };
      setDetails(cache.current);
    }
    const memberIds: string[] = JSON.parse(key)[2];
    if (!expanded || memberIds.length === 0) return;
    const missing = memberIds.filter((id) => !cache.current.members.has(id));
    if (missing.length === 0) return;
    let cancelled = false;
    setDetails({ ...cache.current, loading: true, failed: false });
    void (async () => {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        if (cancelled) return;
        const results = await Promise.allSettled(missing.map(async (id) => {
          const member = await getTaskmasterRecommendation(baseUrl, id);
          if (member.id !== id || member.rollupMemberIds.length > 0) throw new Error("Invalid family detail");
          return member;
        }));
        if (cancelled) return;
        const members = new Map(cache.current.members);
        for (const result of results) {
          if (result.status === "fulfilled") members.set(result.value.id, result.value);
        }
        cache.current = { key, members, loading: false, failed: results.some((result) => result.status === "rejected") };
      } catch {
        if (cancelled) return;
        cache.current = { ...cache.current, loading: false, failed: true };
      }
      setDetails(cache.current);
    })();
    return () => { cancelled = true; };
  }, [expanded, key, retry]);

  const isRollup = recommendation.rollupMemberIds.length > 0;
  const current = details.key === key ? details : undefined;
  return <details className="recommendation-evidence" onToggle={(event) => setExpanded(event.currentTarget.open)}>
    <summary>{isRollup
      ? `Evidence from ${recommendation.rollupMemberIds.length} exact families`
      : `Evidence (${recommendation.evidence.length + (recommendation.repositoryEvidence?.length ?? 0) + (recommendation.knowledgeEvidence?.length ?? 0)})`}</summary>
    {isRollup ? <>
      {current?.loading && <p role="status">Loading family evidence…</p>}
      {current?.failed && <p className="recommendation-error" role="alert">
        Couldn't load all family evidence. <button type="button" onClick={() => setRetry((value) => value + 1)}>Retry</button>
      </p>}
      {recommendation.rollupMemberIds.map((id) => {
        const member = current?.members.get(id);
        return member ? <section key={id} aria-label={`Exact family: ${member.title}`}>
          <h4>{member.title}</h4>
          <EvidenceRows recommendation={member} onSelectSession={onSelectSession} />
        </section> : null;
      })}
    </> : <EvidenceRows recommendation={recommendation} onSelectSession={onSelectSession} />}
  </details>;
}
