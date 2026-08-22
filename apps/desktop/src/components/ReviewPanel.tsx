import { memo, useCallback, useEffect, useRef, useState, type ComponentProps } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import EmptyState from "./EmptyState";

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

// Memoized because ReviewTab re-renders on every ~2s session poll even when
// sessionId is unchanged; react-markdown re-parses its input on every render
// with no internal caching, so without this the whole Markdown pipeline
// would rerun on a timer instead of only when the reviewed content changes.
function ReviewPanel({ sessionId }: { sessionId: string | null }) {
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
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>{content}</ReactMarkdown>
    </div>
  );
}

export default memo(ReviewPanel);
