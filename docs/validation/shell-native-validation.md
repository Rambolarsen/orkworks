---
type: Validation Report
title: Fixed shell native validation
description: Partial macOS native walkthrough evidence and remaining release gates for the fixed desktop shell.
tags: [orkworks, desktop, accessibility, validation]
status: draft
---

# Fixed shell native validation

Tracking: [#836](https://github.com/Rambolarsen/orkworks/issues/836), under
[#755](https://github.com/Rambolarsen/orkworks/issues/755). This report records
partial native UI evidence, not release acceptance or a completed screen-reader
walkthrough. Automated layout fixtures cannot complete the remaining native gates.

The expected behavior comes from the
[application-shell contract](../superpowers/specs/2026-10-05-application-shell-navigation-design.md#keyboard-accessibility-and-motion)
and [MVP shell scope](../../specs/orkworks-mvp.md#electron-desktop-shell).

## Build and observation context

- Source for both runs: `0f0faf5973d465b242915f3e7938332adfa267a9`; no runtime
  source edits. This is a development build containing PR #796 and later shell
  refinements, not a packaged installer or the original #796 head.
- Host: macOS 26.6.2, build 25G83, arm64. Electron 44.6.0; pnpm 11.24.0.
- UI observed and operated through native computer-use accessibility and screenshot
  APIs on 2026-10-14, with continuation on 2026-10-15. These are agent-operated
  native walkthroughs; no human acceptance is implied. Screenshots were inspected
  in the session but are not retained in this repository.
- Owned checkout: `orkworks-shell-native-validation`; launch from `apps/desktop/`
  with `ORKWORKS_DEV_PORT=5373 pnpm dev`, using checkout-scoped settings.
- Disposable Git workspace: `/private/tmp/orkworks-836-native-workspace`.
  Two ordinary Shell sessions were used; no coding-agent harness was launched.
- October 14: initial renderer/native outer measurements were 1400 × 872 at
  100%; native edge resizing produced 902 × 602. At high zoom, read-only DevTools
  measurements returned `devicePixelRatio=4.147200107574463` on the 2× display:
  approximately 207.36% zoom. Native View-menu steps were used, not CSS zoom.
  The sampled intermediate ratio was 2.880000114440918 (144%).
- October 15 restart: 1400 × 874 at 100%, with `devicePixelRatio=2`.
  The prior Shell sessions were remembered history; they were inspected without
  resume. Native window zoom expanded the window, as observed in screenshots.
- DevTools was closed during navigation/layout observations. It was opened only
  for read-only geometry, pixel-ratio and reduced-motion measurements; its docked
  viewport dimensions do not qualify shell breakpoints.
- Screen reader: none used. Accessibility-tree output is not spoken-announcement
  or screen-reader usability evidence.

## Observed checks

Each result below applies only to the stated steps and build.

| Run / conditions | Action and expected result | Actual result / disposition |
| --- | --- | --- |
| Oct 14, 1400 × 872, 100% | View → Sessions; Down/Up changes the selected session while retaining list navigation; Enter focuses its terminal | Down and Up changed the selected row and central subject; Enter reported focus on Terminal input. Observed as expected. |
| Oct 14, 1400 × 872, 100% | Tab follows list actions, Settings, separator and content controls | From the Sessions list, Tab reached the first row's Kill session, the second row's Kill session, Settings, Sessions width splitter, then the Sessions toolbar button. Observed for this segment; not a full tab-order acceptance. |
| Oct 14, 1400 × 872, 100% | Separator arrows change width and Home/End reach bounds | Right: 240 → 256; Left: 256 → 240; Home: 200; End: 320 pixels. AX value text tracked the changes and focus stayed on the splitter. Inspector separator and spoken bounds/orientation remain untested. |
| Oct 14, 1400 × 872, 100% | View → Capacity opens/focuses inspector; repeating closes it and restores prior focus | Capacity heading gained focus; repeat removed the inspector and restored Terminal input focus. Capacity content was the existing unimplemented placeholder. |
| Oct 14, 902 × 602, approximately 207.36% | Details opens as a temporary central page; Escape returns to invoker | Details replaced the terminal, showed Back to Terminal, and focused its heading; Escape restored the terminal and focused Details. Header controls remained visible in the inspected screenshots. |
| Oct 14, narrow/high zoom | Sessions opens as a temporary page; Down selects another session; Enter opens its terminal | Sessions page exposed both rows and Back to Terminal. Down changed selection; Enter returned to that terminal and focused Terminal input. One central surface was visible. |
| Oct 14, resize/zoom and return to 100% | Existing terminal survives hidden presentation and refits; prior output remains | Original session returned after compact Sessions navigation. On return to 100%, screenshot showed the original `836-terminal-before-hide` marker and shell prompt. This is bounded replay/refit evidence, not a stress or output-drain test. |
| Oct 15, 1400 × 874, 100%, OS Reduce motion on | Electron receives OS preference and navigation remains usable | System Settings initially showed Reduce motion off; it was enabled, and renderer `matchMedia('(prefers-reduced-motion: reduce)').matches` returned true. Remembered history and Capacity open/close remained usable. Live attention transitions and motion suppression were not exercised. OS setting was restored to off and verified in System Settings. |
| Oct 15, 100% | Native green-control zoom action changes window size without losing history | Invoked the exposed native “zoom the window” action; screenshot showed the enlarged shell and retained original terminal marker. Minimize/restore, fullscreen and close lifecycle are not accepted by this check. |

## Defects and inconclusive observations

- Fresh-worktree startup initially failed: `fs-ext` had ABI 147 while Electron
  required ABI 149, despite a successful `install-app-deps` step. Forced rebuild
  of only `fs-ext` for Electron 44.6.0 enabled launch and subsequent restart.
  The root cause is not established. Tracked separately in
  [#859](https://github.com/Rambolarsen/orkworks/issues/859); no runtime fix is
  delivered by this report.
- Title-region drag attempts did not change the measured `screenX=56`,
  `screenY=33`, including attempts at 100% and high zoom. This is inconclusive:
  native drag behavior through this automation path has not been qualified.
  A human native drag check is required before treating this as either a pass
  or a product defect.
- Shell startup printed a missing completion-script warning from the host's
  existing `.zshrc`. The prompt remained usable; the host configuration was not
  changed. It is not attributed to OrkWorks.
- Existing keyboard-focus work is tracked by
  [#850](https://github.com/Rambolarsen/orkworks/issues/850). This report does not
  accept its fix or substitute for current-head validation.

## Remaining release evidence

Keep #836 open. None of its broad acceptance checkboxes is fully established
by this partial run.

- Human macOS title drag, minimize/restore, fullscreen and close/control checks,
  including narrow and exact 200% conditions.
- Exact 200% zoom qualification. The recorded native zoom was 207.36%; do not
  relabel it as 200%. Repeat at target zoom with explicit measurement.
- Full keyboard walkthrough, including inspector separator bounds, utility
  return variants, terminal Escape/arrow passthrough, saved custom shortcuts,
  focus visibility and modal interaction.
- Screen-reader walkthrough with reader/version, spoken names, list selection,
  headings, separator orientation/current/min/max, return controls and meaningful
  focus announcements. Native AX visibility alone cannot satisfy this gate.
- Reduced-motion live attention/transition checks, beyond preference detection
  and static navigation, with the original OS setting restored afterward.
- Windows titlebar overlay, reserved controls, Alt menu and native window checks;
  Linux native chrome/window checks, each at narrow width and target zoom.
- Retest any confirmed defects after fixes on an explicitly identified build;
  rerun relevant checks when the shell or build changes. Review, Actions and
  Workflow delivery remain separately gated.


## October 16 continuation

This agent-operated native walkthrough extends the earlier partial evidence. It
is not human acceptance, exact-200% qualification, or a completed platform gate.

### Build and conditions

- Source: `68632e240a5ad47274e760ab65c04cbfc1969ea0`, including the visible
  keyboard-focus fix from PR #852. No runtime source was modified.
- Host: macOS 26.6.2, build 25G83, arm64; Electron 44.6.0; pnpm 11.24.0.
- Owned checkout: `orkworks-shell-native-validation-836`; launched with
  `ORKWORKS_DEV_PORT=5473 pnpm dev`, using checkout-scoped settings.
- Disposable Git workspace: `/private/tmp/orkworks-836-native-20261016`.
  One ordinary Shell session was created. No coding-agent harness was launched
  and no coding-tool integration was enabled.
- Initial read-only DevTools measurements: native outer size 1400 × 870,
  position `(56, 33)`, and `devicePixelRatio=2` on the 2× display. Docked
  DevTools reduced the renderer viewport; it was closed for shell observations.
- Native View-menu zoom steps produced `devicePixelRatio=4.147200107574463`,
  approximately 207.36%. Native edge resizing then produced outer size
  903 × 865 at the same pixel ratio. These conditions do not establish 200%.
- Observations used native AX and screenshots, inspected in this session and
  not retained in the repository. No screen reader was used.

### Observed results

| Conditions | Action and expected result | Actual result / disposition |
| --- | --- | --- |
| 1400 × 870, 100% | Enter/exit native fullscreen without losing the ordinary terminal | `Control+Command+F` expanded the window and removed the exposed native window controls; repeating restored the window and controls. The `836-native-20261016-before-controls` output marker remained visible in both states. Bounded fullscreen/replay check observed as expected. |
| 1400 × 870, 100% | View → Sessions followed by Tab navigation exposes visible keyboard focus | Tab reached Kill session, then Settings. AX reported Settings focused and the screenshot showed its rounded lime outline, along with the Sessions region outline. This validates that sampled focus path on the #852-containing build, not every control or the full keyboard gate. |
| 903 × 865, approximately 207.36% | Native resize preserves reachable compact navigation and terminal content | Window width changed from 1400 to 903; header controls wrapped visibly, native traffic-light controls remained present, and the terminal marker and prompt remained visible. No overlap or unreachable header control was observed in this sample. |
| Same narrow/high-zoom conditions | Details opens a temporary page; Escape returns to its invoker | Details replaced Terminal, exposed Back to Terminal, and focused the Details heading. Escape restored Terminal and AX reported the Details button focused. Observed as expected. |
| After high-zoom checks | View → Actual Size restores ordinary presentation | The Sessions region and width splitter returned and Details remained the focused invoker. This is observed presentation restoration; a second numeric pixel-ratio measurement was not taken. |

### Limits and remaining disposition

- A title-region drag attempt again left measured position `(56, 33)` unchanged.
  Native edge resizing did change window dimensions through the same API, but
  this does not qualify title dragging. Keep the human drag check open; do not
  classify the unchanged position alone as a product defect.
- Invoking the native minimize control did not provide a distinguishable
  minimized state in subsequent app-bound AX/screenshot observations. This
  attempt does not establish minimize/restore behavior.
- The View menu exposes Actual Size, Zoom In and Zoom Out, with no exact-200%
  selection. The measured high-zoom run remains 207.36%; exact 200% is pending.
- Fresh startup encountered the same Node ABI 147 / Electron ABI 149 mismatch
  previously recorded. This build's `build:native` verification detected it,
  rebuilt `fs-ext` for Electron 44.6.0, verified the module, and continued to the
  running app without a manual repair. This observes the current repair path;
  it does not establish the historical root cause or every native-build case.
- The existing host `.zshrc` completion warning appeared again; the Shell
  prompt and marker command worked. No host configuration was changed.
- Windows/Linux chrome, spoken screen-reader announcements, exact 200%,
  minimize/restore, close lifecycle, full keyboard coverage and live
  reduced-motion attention transitions remain unaccepted. Keep #836 open.
