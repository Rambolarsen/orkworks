import type { ResumeChoice } from "../labels";
import { VOCAB } from "../labels";

interface ResumeChooserProps {
  options: ResumeChoice[];
  onSelect: (option: ResumeChoice) => void;
}

function ResumeOptionGlyph({ recommended, unavailable }: { recommended?: boolean; unavailable?: boolean }) {
  if (recommended && !unavailable) {
    return (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M20 6 9 17l-5-5" />
      </svg>
    );
  }
  if (unavailable) {
    return (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    );
  }
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" aria-hidden="true">
      <circle cx="12" cy="12" r="4" fill="currentColor" stroke="none" />
    </svg>
  );
}

/**
 * The one app-only move a dead/blocked session offers — the terminal can't
 * reopen a process that isn't running. Unavailable options stay visible
 * (dimmed, with a reason) rather than being hidden.
 */
function ResumeChooser({ options, onSelect }: ResumeChooserProps) {
  return (
    <div className="resume-chooser">
      <div className="resume-chooser-title">{VOCAB.resumeChooserTitle}</div>
      {options.map((option, i) => {
        const className = [
          "resume-option",
          option.recommended ? "resume-option--recommended" : "",
          option.unavailable ? "resume-option--unavailable" : "",
        ].filter(Boolean).join(" ");
        const content = (
          <>
            <span className="resume-option-icon">
              <ResumeOptionGlyph recommended={option.recommended} unavailable={option.unavailable} />
            </span>
            <span className="resume-option-body">
              <span className="resume-option-label">{option.label}</span>
              {option.sub && <span className="resume-option-sub">{option.sub}</span>}
            </span>
            {option.recommended && !option.unavailable && (
              <span className="resume-option-tag">{VOCAB.resumeBestTag}</span>
            )}
          </>
        );
        if (option.unavailable) {
          return (
            <div key={i} className={className} aria-disabled="true">
              {content}
            </div>
          );
        }
        return (
          <button key={i} type="button" className={className} onClick={() => onSelect(option)}>
            {content}
          </button>
        );
      })}
    </div>
  );
}

export default ResumeChooser;
