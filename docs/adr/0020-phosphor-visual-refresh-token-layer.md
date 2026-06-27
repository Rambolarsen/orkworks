# Phosphor visual refresh: cool-graphite + lime token layer

- Status: accepted
- Deciders: Lars-Erik Arnesen
- Date: 2026-06-27

## Context

The shipped desktop UI was a warm-dark theme with a steel-blue accent and tight
2–6px spacing/radius metrics. A Dieter Rams design audit (`DESIGN-IS-2026-06-19/`)
flagged the substrate — ad-hoc color values, an inconsistent spacing rhythm, and
missing semantic layers — as the root cause of a dated, slightly muddy look, and
motivated a real token layer, a plain-language label module, and fuller state
coverage.

That work was developed into a standalone design system — the **"Phosphor"
refresh** — captured as a claude.ai/design design-system project ("OrkWorks
Design System"). Phosphor reinterprets the app as a cool-graphite dev tool in the
Linear / Raycast / Warp lineage: near-black graphite surfaces with a faint blue
cast, vibrant **ork-lime** as the primary UI accent (solid lime fills with dark
text), a clean Primer-style four-hue state palette, roomier 4px-based spacing,
softer 6–16px radii, and real elevation/motion/focus tokens. It is a deliberate
modernization that goes beyond the shipped build; the information architecture,
vocabulary, and component contracts are unchanged.

The open question was whether to treat Phosphor as the app's visual direction and
migrate the live token layer to it, versus keeping the warm-dark/steel-blue look.

## Decision

Adopt Phosphor as the canonical visual direction and implement it as a **token
substrate refresh**, not a layout or component-contract change.

- `apps/desktop/src/styles/tokens.css` carries the full Phosphor token layer
  (color / type / spacing / effects). It stays a **single file** so the existing
  `main.tsx` import and the token-presence tests continue to read definitions
  directly; splitting into `@import`ed partials would break those contracts.
- All previously-defined token names are preserved (re-pointed at new raw ramps),
  so every existing `var(--…)` consumer in `App.css` and the components keeps
  resolving. The refresh lands primarily through the token values, plus migrating
  the last hardcoded `App.css` values (radii, shadows, scrims, mono font, eyebrow
  tracking, font-sizes) onto tokens. `App.css` remains hex-free.
- The **dark** theme is the source of truth. A `[data-theme="light"]` token block
  ships as a faithful derivation, but no runtime toggle / OS-following is wired up
  (tracked separately); the app stays dark-only in behavior for now.
- Terminals stay dark in both themes; the xterm ANSI palette
  (`terminalTheme.ts` / `--term-*`) is theme-independent.

## Consequences

- New OrkWorks surfaces, mocks, and marketing can be built on-brand against the
  semantic tokens, and the claude.ai/design project stays a usable reference (via
  the DesignSync tooling).
- Because the change is a token swap, future palette/spacing/radius adjustments
  propagate from one file; authoring against raw hex in component CSS is now a
  regression the no-hex test guards against.
- The semantic change to `--text-on-accent` (now dark ink for the lime fill) made
  any "light text on a dark accent surface" pairing wrong; such cases must use the
  solid-lime primary fill instead (the resume button was fixed accordingly).
  Reviewers should watch for this pairing in new code.
- A real light theme is now one decision away (the tokens exist), but turning it on
  is net-new product behavior and warrants its own design pass before wiring.
- This ADR records a visual-direction decision; it does not change the
  single-active-context primitive (ADR 0013) or any other architectural boundary.
