import { unverifiedAdviceWarning } from '../advice.ts';
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

const EMPTY_STATES = {
  ready: { title: "No current actions", detail: "There are no session items or workspace recommendations to inspect." },
  loading: { title: "Loading actions", detail: "Waiting for the remaining source information." },
  partial: { title: "No actions in available data", detail: "Only part of the source information is available." },
  unavailable: { title: "Actions unavailable", detail: "Some source information could not be loaded." },
};

export default function ActionsOverview({
  model, focusRequest, inspected, onInspectAction, onFocusHandled, onOpenSession, onOpenRecommendation,
}: Props) {
  const root = useRef<HTMLDivElement>(null);
  const current = inspected && !inspected.unavailable
    ? model.rows.find(row => row.key === inspected.key && row.revision === inspected.revision && !row.unavailable)
    : undefined;
  const lastFocusRequest = useRef(0);
  const statuses = [model.sessionsStatus, model.recommendationsStatus];
  const emptyState = EMPTY_STATES[statuses.includes("unavailable") ? "unavailable"
    : statuses.includes("loading") ? "loading" : statuses.includes("partial") ? "partial" : "ready"];

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
    <dl className="actions-source-summary" aria-label="Action sources">
      <div><dt>Session attention</dt><dd>{model.sessionCountValue}</dd></div>
      <div><dt>Recommendations</dt><dd>{model.recommendationCountValue}</dd></div>
    </dl>
    {!model.rows.length && <div className="actions-empty" role="status">
      <h3>{emptyState.title}</h3>
      <p>{emptyState.detail}</p>
    </div>}
    {[...new Set(model.rows.map(row => row.group))].map(group => <section key={group} aria-label={group}>
      <h3>{group}</h3>
      {model.rows.filter(row => row.group === group).map(row => {
        const sourceLabel = (row.source === "session" ? "Session" : "Recommendation")
          + (row.unavailable ? " · Unavailable — refresh pending" : "");
        return <div key={row.key} className="actions-row">
          {row.unverifiedAdvice && <p className="advice-warning" role="note">{unverifiedAdviceWarning}</p>}
          <button type="button" data-action-key={row.key} aria-pressed={current?.key === row.key}
            aria-label={row.title} aria-description={sourceLabel} onClick={() => onInspectAction(row)}>
            <span className="actions-row-title">{row.title}</span>
            <span className="actions-row-source" aria-hidden="true">{sourceLabel}</span>
          </button>
        </div>;
      })}
    </section>)}
    {inspected && !current && <div className="actions-row" role="status">
      <span>{inspected.unverifiedAdvice ? 'Inspected recommendation' : inspected.title} · Inspected action unavailable or changed. Inspect a current row for updated details.</span>
    </div>}
    {current && <section data-actions-detail aria-label="Inspected action">
      {current.unverifiedAdvice && <p className="advice-warning" role="note">{unverifiedAdviceWarning}</p>}
      <h3>{current.title}</h3>
      <p>{current.summary || "No additional details reported."}</p>
      {current.source === "session"
        ? <button type="button" onClick={() => onOpenSession(current.id)}>Open terminal/history</button>
        : <button type="button" onClick={() => onOpenRecommendation(current.id)}>Open recommendation</button>}
    </section>}
  </div>;
}
