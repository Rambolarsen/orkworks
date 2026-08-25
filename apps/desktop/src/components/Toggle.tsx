interface ToggleProps {
  checked: boolean;
  onChange: () => void;
  label?: string;
  disabled?: boolean;
}

/** Pill switch used throughout Settings in place of native checkboxes. */
export default function Toggle({ checked, onChange, label, disabled }: ToggleProps) {
  const button = (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label ?? undefined}
      className={`ui-toggle${checked ? " ui-toggle--on" : ""}`}
      onClick={onChange}
      disabled={disabled}
    >
      <span className="ui-toggle-thumb" />
    </button>
  );

  if (!label) return button;

  return (
    <label className="ui-toggle-row">
      {button}
      <span className="ui-toggle-label">{label}</span>
    </label>
  );
}
