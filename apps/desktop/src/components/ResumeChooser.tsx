import { Check, CircleSlash } from "lucide-react";
import type { ResumeChoice } from "../labels";
import { VOCAB } from "../labels";

interface ResumeChooserProps {
  options: ResumeChoice[];
  onSelect: (option: ResumeChoice) => void;
}

/** No lucide icon renders a small centered dot at this weight, so this one glyph stays hand-rolled. */
function DefaultOptionGlyph() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="4" fill="currentColor" />
    </svg>
  );
}

function ResumeOptionGlyph({ recommended, unavailable }: { recommended?: boolean; unavailable?: boolean }) {
  if (recommended && !unavailable) return <Check size={14} strokeWidth={2.4} aria-hidden="true" />;
  if (unavailable) return <CircleSlash size={14} strokeWidth={2.4} aria-hidden="true" />;
  return <DefaultOptionGlyph />;
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
