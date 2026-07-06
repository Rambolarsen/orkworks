import { HARNESS_ICON_PATHS, harnessIconKey } from "../harnessIcons";

/**
 * A tiny glyph identifying which coding tool a session is running — for the
 * at-a-glance scan the Sessions list needs. Uses each tool's official
 * monochrome mark so rows are scannable by shape, not just by reading a
 * label. Deliberately grayscale — attention tone already owns color on the
 * row via the StatusIndicator, so the tool mark reads as metadata, not
 * status. Unrecognized tool names fall back to a generic terminal-prompt
 * glyph, so new coding tools never render blank.
 */

const FALLBACK = <path d="M4 7l6 5-6 5M13 19h7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" fill="none" />;

interface HarnessIconProps {
  /** Coding-tool display name or harness id; render nothing when unknown. */
  tool?: string;
  size?: number;
}

function HarnessIcon({ tool, size = 15 }: HarnessIconProps) {
  if (!tool) return null;
  const paths = HARNESS_ICON_PATHS[harnessIconKey(tool)];

  return (
    <svg
      role="img"
      aria-label={tool}
      className="harness-icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
    >
      <title>{tool}</title>
      {paths
        ? paths.map((d, i) => <path key={i} fillRule="evenodd" clipRule="evenodd" d={d} />)
        : FALLBACK}
    </svg>
  );
}

export default HarnessIcon;
