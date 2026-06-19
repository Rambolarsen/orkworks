# 02 — Scorecard

Scores follow the per-principle rubric anchors in the skill. Tie-breaker = pick lower. Score worst, not mean. Evidence anchors reference `01-evidence.md` (S/V/C/W/A/G prefixes).

---

1. **Good design is innovative — Score: 1/3**
   Evidence: Standard VS-Code-style Dockview + xterm panes; the observability framing (source-confidence badges, peon vs agent provenance) exists at the data layer but is rendered as raw enums so the differentiator isn't legible (C2, S6).
   Justification: Imitates competitors (terminal multiplexer / IDE chrome) with minor variation; the genuinely novel concept (provenance-tagged metadata) isn't visually expressed.

2. **Good design makes a product useful — Score: 2/3**
   Evidence: Primary task is situational awareness across N sessions, with deliberate context-switch when acting. The IA fits the model: sessions list = multi-view, single active terminal = correct context primitive, detail panel = secondary context for the focused session. Friction is in the *adjacent surface*: 2 of 5 default panels are unshipped placeholders (S9); the sessions-list dashboard renders raw enum status (`waiting_for_input`, `agent · 100%`) that defeats glanceability (C2); silent error swallowing means failed actions give no feedback (C7).
   Justification: Primary task completes via the right primitives, but adjacent surface adds steps and degrades the at-a-glance read.

3. **Good design is aesthetic — Score: 0/3**
   Evidence: Zero design tokens at `:root` (V1); ~50 distinct color literals and ~118 hardcoded hex occurrences across CSS + inline styles (V2); 16 distinct spacing values (V3); 10 orphan single-use colors; 4 divergent empty-state designs (S5).
   Justification: No visible system — colors, spacing, type are literal-everywhere with no shared token layer; not "≤2 minor inconsistencies" but a structural absence of a system.

4. **Good design makes a product understandable — Score: 1/3**
   Evidence: Raw enum leaks (`waiting_for_input`, `latest_cwd`, `unsupported`, `agent · 100%` with no "confidence" label) — C2; jargon "Peon", "backend:", roadmap codes "M8"/"M9" — C2; glyph-only `⇄`, `+`, `×`, `⚠` controls relying on hover tooltips — C5; same field labeled "Task" in one place and "Summary" in another — C3.
   Justification: Multiple controls unclear AND jargon pervasive; a first-time user could not name several controls without explanation.

5. **Good design is unobtrusive — Score: 1/3**
   Evidence: 5 panels visible by default on first launch (W6); 2 of them are placeholders (S9); "Mission Control for AI Agents" decorative tagline baked into product chrome (C1, G5); Dockview tab + sash chrome surrounds all content.
   Justification: Chrome competes with content — five panels of frame around a single-session terminal, two of which carry no signal, plus a marketing tagline as placeholder content.

6. **Good design is honest — Score: 1/3**
   Evidence: One inflation — "Mission Control for AI Agents" (C1); one label/behavior mismatch — empty-state hint "Open one from the View menu" paired with a "Reset Layout" button that opens all five panels (C4); naming drift "Open Folder" vs "Switch workspace" for the same action (C3).
   Justification: One inflation plus a mismatched recovery affordance — beyond the "≤1 minor inflation" bar.

7. **Good design is long-lasting — Score: 2/3**
   Evidence: Dark VS-Code-ish palette with no skeuomorph, gradients, or trend typography; "Mission Control" copy is mildly dated startup-voice but visuals are conservative.
   Justification: One dated marker (the tagline phrasing); otherwise the visual language would read as current in 3 years.

8. **Good design is thorough down to the last detail — Score: 1/3**
   Evidence: Zero `:focus` / `:focus-visible` rules; outline actively removed on the sessions list (V5); no loading skeletons or spinners; errors silently swallowed in 5 handlers (C7); status badge has no distinct error variant (V6).
   Justification: Three states broadly missing — focus, loading, error — across the primary surfaces.

9. **Good design is environmentally friendly — Score: 1/3**
   Evidence: 842 KB single JS chunk, no code-splitting (W1); xterm cursor blink runs without consulting `prefers-reduced-motion` (W5); app ignores `prefers-color-scheme` and is dark-only (W5).
   Justification: 500KB–2MB band with motion always on and no light-mode path; squarely in the "1" anchor.

10. **Good design is as little design as possible — Score: 0/3**
    Evidence: 3 dead components shipping (`RightSidebar`, `LeftSidebar`, `TerminalTabs`) (S2); 2 panels rendering placeholder copy for unshipped milestones (S9); 4 divergent empty-state designs for the same idle (S5); two divergent default-layout builders likely to drift (S4); duplicated session-detail content across live + dead files (S3).
    Justification: Page is dominated by duplicated affordances and dead/placeholder elements — well beyond 5 removable items.

---

**Total: 10 / 30**
