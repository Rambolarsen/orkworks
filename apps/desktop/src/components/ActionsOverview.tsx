import { useLayoutEffect, useRef } from "react";
import type { ActionRow, ActionsOverviewModel } from "../actionsOverview";

interface Props {
  model: ActionsOverviewModel;
  focusRequest: number;
  inspected: ActionRow | null;
  onInspectAction: (row: ActionRow | null) => void;
  onFocusHandled: () => void;
  onOpenSession: (id: string) => void;
  onOpenRecommendation: (id: string) => void;
}

export default function ActionsOverview({
  model, focusRequest, inspected, onInspectAction, onFocusHandled, onOpenSession, onOpenRecommendation,
}: Props) {
  const root = useRef<HTMLDivElement>(null);
  const current = inspected && !inspected.unavailable
    ? model.rows.find(row => row.key === inspected.key && row.revision === inspected.revision && !row.unavailable)
    : undefined;
  const lastFocusRequest = useRef(0);

  useLayoutEffect(() => {
    if (!focusRequest || lastFocusRequest.current === focusRequest) return;
    lastFocusRequest.current = focusRequest;
    const first = model.rows.find(row => !row.unavailable);
    onInspectAction(first ?? null);
    onFocusHandled();
    const row = Array.from(root.current?.querySelectorAll<HTMLElement>("[data-action-key]") ?? [])
      .find(element => element.dataset.actionKey === first?.key);
    if (row) {
      row.focus({ preventScroll: true });
      row.scrollIntoView({ block: "nearest" });
    } else {
      root.current?.focus({ preventScroll: true });
    }
  }, [focusRequest, model, onInspectAction, onFocusHandled]);

  useLayoutEffect(() => {
    // Once stale, an inspection requires a new user choice even if identical bytes return.
    if (inspected && !inspected.unavailable && !current && !focusRequest) {
      onInspectAction({ ...inspected, unavailable: true });
    }
  }, [inspected, current, focusRequest, onInspectAction]);

  return <div className="actions-overview" ref={root} tabIndex={-1}>
    <p>Session attention and workspace recommendations.</p>
    <p>{model.sessionCountLabel} · {model.recommendationCountLabel}</p>
    {!model.rows.length && <p role="status">
      {model.sessionCount === null || model.recommendationCount === null
        ? "Actions unavailable or still loading. See source status above."
        : "No current actions."}
    </p>}
    {[...new Set(model.rows.map(row => row.group))].map(group => <section key={group} aria-label={group}>
      <h3>{group}</h3>
      {model.rows.filter(row => row.group === group).map(row => <div key={row.key} className="actions-row">
        <button type="button" data-action-key={row.key} aria-pressed={current?.key === row.key}
          onClick={() => onInspectAction(row)}>{row.title}</button>
        <span>{row.source === "session" ? "Session" : "Recommendation"}
          {row.unavailable ? " · Unavailable — refresh pending" : ""}</span>
      </div>)}
    </section>)}
    {inspected && !current && <div className="actions-row" role="status">
      <span>{inspected.title} · Inspected action unavailable or changed. Inspect a current row for updated details.</span>
    </div>}
    {current && <section data-actions-detail aria-label="Inspected action">
      <h3>{current.title}</h3>
      <p>{current.summary || "No additional details reported."}</p>
      {current.source === "session"
        ? <button type="button" onClick={() => onOpenSession(current.id)}>Open terminal/history</button>
        : <button type="button" onClick={() => onOpenRecommendation(current.id)}>Open recommendation</button>}
    </section>}
  </div>;
}
