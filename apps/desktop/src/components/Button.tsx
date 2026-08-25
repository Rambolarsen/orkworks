import type { ReactNode } from "react";

interface ButtonProps {
  variant?: "primary" | "secondary" | "ghost";
  size?: "sm" | "md";
  type?: "button" | "submit";
  disabled?: boolean;
  onClick?: () => void;
  ariaLabel?: string;
  children: ReactNode;
}

/** Shared button styling (primary/secondary/ghost) for Settings and other config surfaces. */
export default function Button({ variant = "secondary", size = "md", type = "button", disabled, onClick, ariaLabel, children }: ButtonProps) {
  return (
    <button
      type={type}
      className={`ui-button ui-button--${variant} ui-button--${size}`}
      disabled={disabled}
      onClick={onClick}
      aria-label={ariaLabel}
    >
      {children}
    </button>
  );
}
