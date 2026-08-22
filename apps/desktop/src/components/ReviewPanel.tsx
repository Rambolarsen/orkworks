import { useCallback, useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import EmptyState from "./EmptyState";

export default function ReviewPanel({ sessionId }: { sessionId: string | null }) {
  const [content, setContent] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const requestId = useRef(0);
  const load = useCallback(() => {
    if (!sessionId) return;
    const currentRequest = ++requestId.current;
    setContent(null);
    setError(false);
    void window.orkworks.getPlanContent(sessionId)
      .then((value) => { if (currentRequest === requestId.current) setContent(value); })
      .catch(() => { if (currentRequest === requestId.current) setError(true); });
  }, [sessionId]);

  useEffect(() => {
    load();
  }, [load]);

  if (!sessionId) return <EmptyState message="Select a session with a plan to review it." />;
  if (error) return <EmptyState message="This plan is no longer available." action={{ label: "Retry", onClick: load }} />;
  if (content === null) return <EmptyState message="Loading plan…" />;
  return (
    <div className="review-plan-content">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
    </div>
  );
}
