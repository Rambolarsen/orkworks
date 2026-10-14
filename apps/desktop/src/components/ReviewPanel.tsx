import { memo, useRef, useState, type ComponentProps } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import EmptyState from "./EmptyState";
import { normalizeMarkdownLinkDestinations } from "../markdownLinks";

// A same-document "#fragment" link is safe to leave to the browser's native
// handling (in-page scroll, no navigation event). Anything else — a relative
// path, an absolute path, or a full URL — must not be allowed to navigate
// the renderer directly: Electron's will-navigate guard in
// electron/externalLinks.ts only blocks cross-origin navigation, so a
// same-origin relative link (e.g. to another spec file, common in these
// docs) would otherwise replace the app window instead of doing nothing.
// Route it through the same openExternalLink bridge terminal links use,
// which only ever acts on http(s) URLs and silently no-ops everything else.
function ReviewLink({ href, children }: ComponentProps<"a">) {
  if (href?.startsWith("#")) return <a href={href}>{children}</a>;
  return (
    <a
      href={href}
      onClick={(event) => {
        event.preventDefault();
        if (href) void window.orkworks.openExternalLink(href);
      }}
    >
      {children}
    </a>
  );
}

const markdownComponents: Components = { a: ReviewLink };

interface ReviewPanelProps {
  sessionId: string;
  content: string | null;
  loading: boolean;
  error: boolean;
  changed: boolean;
  canRequest: boolean;
  onRefresh: () => void;
  onRequest: (sessionId: string) => Promise<void>;
}

function ReviewPanel({ sessionId, content, loading, error, changed, canRequest, onRefresh, onRequest }: ReviewPanelProps) {
  const [requesting, setRequesting] = useState(false);
  const [requestError, setRequestError] = useState<string | null>(null);
  const pending = useRef(false);
  const requestReview = () => {
    if (pending.current || !canRequest || loading || error || content === null) return;
    pending.current = true;
    setRequesting(true);
    setRequestError(null);
    void onRequest(sessionId).catch(() => {
      setRequestError("Couldn't request independent review. Try again.");
    }).finally(() => { pending.current = false; setRequesting(false); });
  };
  return <>
    <div className="review-actions">
      <button type="button" data-review-refresh disabled={loading} onClick={onRefresh}>Refresh plan</button>
      {canRequest && <button type="button" data-review-request disabled={requesting || loading || error || content === null} onClick={requestReview}>
        {requesting ? "Requesting review…" : "Request independent review"}
      </button>}
      {requestError && <span role="alert">{requestError}</span>}
    </div>
    {changed && !loading && !error && <p className="review-change-notice" role="status">Plan changed. Showing refreshed content.</p>}
    {error ? <EmptyState message="This plan is no longer available." action={{ label: "Retry", onClick: onRefresh }} />
      : content === null ? <EmptyState message="Loading plan…" /> : (
    <div className="review-plan-content">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>{normalizeMarkdownLinkDestinations(content)}</ReactMarkdown>
    </div>
  )}</>;
}

export default memo(ReviewPanel);
