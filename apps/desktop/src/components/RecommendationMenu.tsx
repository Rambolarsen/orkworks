import { ChevronDown, MoreHorizontal } from 'lucide-react';
import { useId, type ReactNode, type SyntheticEvent } from 'react';

/** Native disclosure keeps actions in normal keyboard order without claiming menu-role semantics. */
export default function RecommendationMenu({ label, children, primary = false }: { label: string; children: ReactNode; primary?: boolean }) {
  const descriptionId = useId();
  const close = (element: HTMLDetailsElement, restoreFocus: boolean) => {
    element.open = false;
    if (restoreFocus) element.querySelector('summary')?.focus();
  };
  return <details className={`recommendation-menu${primary ? ' recommendation-menu--primary' : ''}`}
    onKeyDown={event => {
      if (event.key !== 'Escape' || !event.currentTarget.open) return;
      event.preventDefault();
      event.stopPropagation();
      close(event.currentTarget, true);
    }}
    onBlur={event => {
      if (!event.currentTarget.contains(event.relatedTarget)) close(event.currentTarget, false);
    }}
    onClick={(event: SyntheticEvent<HTMLDetailsElement>) => {
      const button = (event.target as HTMLElement).closest('button');
      if (button && !button.disabled) close(event.currentTarget, true);
    }}>
    <summary aria-label={label} title={label} aria-describedby={descriptionId}>{primary ? <ChevronDown size={16} aria-hidden="true" /> : <MoreHorizontal size={18} aria-hidden="true" />}</summary>
    <div className="recommendation-menu-content">
      <p id={descriptionId} className="recommendation-menu-label">{label}</p>
      {children}
    </div>
  </details>;
}
