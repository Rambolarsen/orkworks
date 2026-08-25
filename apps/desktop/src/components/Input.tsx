import type { ChangeEvent, CSSProperties, FocusEvent } from "react";

interface InputProps {
  label?: string;
  type?: "text" | "number";
  value: string | number;
  placeholder?: string;
  onChange?: (e: ChangeEvent<HTMLInputElement>) => void;
  onBlur?: (e: FocusEvent<HTMLInputElement>) => void;
  list?: string;
  min?: number;
  max?: number;
  disabled?: boolean;
  style?: CSSProperties;
}

/** Labeled text/number field shared by the provider and retention sections. */
export default function Input({ label, style, ...inputProps }: InputProps) {
  const input = <input className="ui-input" style={style} {...inputProps} />;

  if (!label) return input;

  return (
    <label className="ui-input-field">
      <span className="ui-input-label">{label}</span>
      {input}
    </label>
  );
}
