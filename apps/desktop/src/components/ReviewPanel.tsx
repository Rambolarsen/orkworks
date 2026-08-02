import { useCallback, useEffect, useState } from "react";
import EmptyState from "./EmptyState";

export default function ReviewPanel({ sessionId }: { sessionId: string | null }) {
  const [content, setContent] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const load = useCallback(() => {
    if (!sessionId) return;
    setContent(null);
    setError(false);
    let current = true;
    void window.orkworks.getPlanContent(sessionId)
      .then((value) => { if (current) setContent(value); })
      .catch(() => { if (current) setError(true); });
    return () => { current = false; };
  }, [sessionId]);

  useEffect(() => {
    return load();
  }, [load]);

  if (!sessionId) return <EmptyState message="Select a session with a plan to review it." />;
  if (content === null) return <EmptyState message="Loading plan…" />;
  if (error) return <EmptyState message="This plan is no longer available." action={{ label: "Retry", onClick: load }} />;
  return <pre className="review-plan-content">{content}</pre>;
}
