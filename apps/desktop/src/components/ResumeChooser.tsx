import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { AlertTriangle, Check, ChevronDown, LoaderCircle, Play } from "lucide-react";
import type { ResumeChoice } from "../labels";

interface ResumeChooserProps {
  options: ResumeChoice[];
  onSelect: (option: ResumeChoice) => void;
  header?: boolean;
  pending?: boolean;
}

export default function ResumeChooser({ options, onSelect, header = false, pending: resuming = false }: ResumeChooserProps) {
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState<ResumeChoice | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const arrow = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const warning = useRef<HTMLDialogElement>(null);
  const id = useId();
  const preferred = options.find(option => option.recommended && !option.unavailable)
    ?? options.find(option => !option.unavailable);

  useLayoutEffect(() => {
    if (!header || !open || !panel.current) return;
    const bounds = panel.current.getBoundingClientRect();
    const shift = bounds.left < 12 ? 12 - bounds.left : Math.min(0, window.innerWidth - 12 - bounds.right);
    panel.current.style.transform = `translateX(${shift}px)`;
  }, [open, header]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const resize = () => setOpen(false);
    document.addEventListener("pointerdown", outside);
    window.addEventListener("resize", resize);
    return () => { document.removeEventListener("pointerdown", outside); window.removeEventListener("resize", resize); };
  }, [open]);
  useEffect(() => {
    if (!pending) return;
    const dialog = warning.current;
    dialog?.showModal();
    return () => { if (dialog?.open) dialog.close(); };
  }, [pending]);
  const optionsSignature = JSON.stringify(options);
  useEffect(() => {
    setOpen(false);
    setPending(null);
  }, [optionsSignature]);

  const choose = (option: ResumeChoice) => {
    if (resuming || option.unavailable) return;
    setOpen(false);
    trigger.current?.focus();
    if (option.strategy === "exact") onSelect(option);
    else setPending(option);
  };
  const cancel = () => { setPending(null); trigger.current?.focus(); };

  return <div className={header ? "resume-menu" : "resume-menu resume-menu--details"} ref={root} onKeyDown={event => {
    if (event.key === "Escape" && open) {
      event.preventDefault(); event.stopPropagation(); setOpen(false); arrow.current?.focus();
    }
  }} onBlur={event => {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOpen(false);
  }}>
    <div className="resume-split-button">
      <button ref={trigger} type="button" className="shell-resume-button" data-shell-resume={header ? true : undefined}
        disabled={resuming || !preferred} aria-busy={resuming} title={preferred?.sub} onClick={() => preferred && choose(preferred)}>
        {resuming ? <><LoaderCircle size={15} className="status-indicator-spin" aria-hidden="true" />Resuming…</> : <><Play size={15} aria-hidden="true" />{preferred?.strategy === "exact" ? "Resume session" : preferred?.label ?? "Resume unavailable"}</>}
      </button>
      <button ref={arrow} type="button" className="resume-menu-arrow" data-resume-toggle disabled={resuming} aria-label="Resume options"
        aria-expanded={open} aria-controls={open ? `${id}-options` : undefined} onClick={() => setOpen(value => !value)}>
        <ChevronDown size={14} aria-hidden="true" />
      </button>
    </div>
    {open && <div ref={panel} id={`${id}-options`} className="resume-menu-popover" data-resume-options role="group" aria-label="Resume options">
      {options.map(option => <button key={option.strategy} type="button" className="resume-option"
        data-resume-strategy={option.strategy} disabled={resuming || option.unavailable} onClick={() => choose(option)}>
        <span className="resume-option-body"><span className="resume-option-label">{option.label}{option.recommended && !option.unavailable && <Check size={14} aria-label="Preferred" />}</span>
          <span className="resume-option-sub">{option.sub}</span></span>
      </button>)}
    </div>}
    {pending && createPortal(<dialog ref={warning} className="resume-warning" data-resume-warning aria-labelledby={`${id}-warning-title`}
      onCancel={event => { event.preventDefault(); cancel(); }} onKeyDown={event => event.stopPropagation()}>
      <h2 id={`${id}-warning-title`}><AlertTriangle size={20} aria-hidden="true" />This may open a different session</h2>
      <p><strong>{pending.label}</strong> resumes the newest conversation {pending.strategy === "latest_cwd" ? "in this folder" : "in this repository"}. It may not be the conversation selected in OrkWorks.</p>
      <p>Use exact resume when you need this specific conversation.</p>
      <footer><button type="button" autoFocus onClick={cancel}>Cancel</button><button type="button" data-resume-confirm disabled={resuming} onClick={() => {
        const current = options.find(option => option.strategy === pending.strategy && !option.unavailable);
        setPending(null);
        if (current && !resuming) onSelect(current);
      }}>{pending.label}</button></footer>
    </dialog>, document.body)}
  </div>;
}
