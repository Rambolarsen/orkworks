import { useRef } from "react";

interface Props {
  label: string;
  value: number;
  minimum: number;
  maximum: number;
  direction?: 1 | -1;
  onChange: (width: number) => void;
}

export default function RegionSeparator({ label, value, minimum, maximum, direction = 1, onChange }: Props) {
  const drag = useRef<{ x: number; width: number } | null>(null);
  const change = (next: number) => onChange(Math.round(Math.max(minimum, Math.min(maximum, next))));
  return <div role="separator" aria-label={`${label} width`} aria-orientation="vertical"
    aria-valuemin={minimum} aria-valuemax={maximum} aria-valuenow={value}
    aria-valuetext={`${value} pixels`} tabIndex={0} className="shell-region-separator"
    onPointerDown={event => {
      if (event.button !== 0) return;
      drag.current = { x: event.clientX, width: value };
      event.currentTarget.setPointerCapture(event.pointerId);
      event.preventDefault();
    }}
    onPointerMove={event => {
      if (drag.current) change(drag.current.width + direction * (event.clientX - drag.current.x));
    }}
    onPointerUp={event => {
      drag.current = null;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onPointerCancel={() => { drag.current = null; }}
    onLostPointerCapture={() => { drag.current = null; }}
    onKeyDown={event => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Home") change(minimum);
      else if (event.key === "End") change(maximum);
      else change(value + (event.key === "ArrowLeft" ? -16 : 16) * direction);
    }} />;
}
