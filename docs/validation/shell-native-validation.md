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


## October 22 continuation

This agent-operated run adds live reduced-motion and inspector-keyboard evidence.
It does not complete #836 or establish human, screen-reader or release acceptance.

### Build and conditions

- Source: `e40889d7552d90c4c8d7bd3aad677f26fd3298c9`; no runtime source edits.
- Host: macOS 26.6.2, build 25G83, arm64; Electron 44.6.0;
  desktop dependency installation used pnpm 11.24.0.
- Owned checkout: `orkworks-shell-native-validation-836-oct22`; launched from
  `apps/desktop/` with `ORKWORKS_DEV_PORT=5573 pnpm dev`, using checkout-scoped
  settings. The observed renderer URL was `http://localhost:5573/`.
- Disposable Git workspace: `/private/tmp/orkworks-836-native-20261022`.
  One ordinary Shell session was created; no coding-agent harness was launched
  and no coding-tool integration was enabled.
- Read-only DevTools measurement: native outer size 1400 × 876, position
  `(56, 33)`, `devicePixelRatio=2` at 100% on the 2× display. DevTools was closed
  for ordinary shell screenshots and docked at the bottom for status measurement.
  Docked viewport dimensions are not breakpoint-acceptance evidence.
- Native AX and screenshots were inspected in the session, not retained in the
  repository. Reduce motion and VoiceOver both started off and were verified
  off again after their respective checks. VoiceOver Utility's caption-panel
  option was already enabled and was not changed.

### Observed results

| Conditions | Action and expected result | Actual result / disposition |
| --- | --- | --- |
| 100%, OS Reduce motion on | Run a bounded Shell loop producing one output line per second; retain Working meaning without the spinning status animation | The live row exposed Working. Read-only computed-style measurement returned `reduce: true`, animation name `none`, duration `1e-05s`, and accessible label `Working`. This establishes the sampled live Working indicator, not Needs You/Blocked/Done transitions or every motion cue. |
| Same workload, OS preference restored off | Original animation returns without losing its label | Measurement returned `reduce: false`, animation name `status-indicator-spin`, duration `1s`. System Settings separately showed Reduce motion off. |
| 1400 × 876, 100%, Details inspector | Shift+Tab from the Details heading reaches Inspector width; Home/End and arrows resize while retaining focus | Inspector width changed 320 → 280 with Home, 280 → 420 with End, 420 → 404 with Right, and 404 → 420 with Left. AX retained splitter focus and reported each pixel value. |
| Same inspector | Tab reaches Pin inspector, then Close inspector; Enter closes and restores the invoker | Focus reached both controls in that order. Enter removed the inspector and focused Details. This is a sampled keyboard path, not the complete tab-order gate. |
| 100%, Sessions sidebar visible | Separator exposes its current range and orientation | Read-only DOM inspection returned label `Sessions width`, orientation `vertical`, minimum 200, maximum 320, current value 240. This is semantic-property evidence, not a spoken announcement. |
| 100%, terminal owns input | Escape and Up reach a Shell `cat -v` process | Terminal output displayed `^[^[[A`. An initial attempt to keep Details beside it was invalidated by expected unpinned-inspector outside-click dismissal; inspection confirmed that dismissal happened when focusing the terminal, before Escape. This does not qualify the pinned-inspector or compact-page interaction. |
| 100%, native close and subsequent automation reactivation | Observe window reconstruction and retained terminal content | A new renderer appeared with no selected session and the same existing Shell row. Explicitly selecting it replayed the `836-motion-live-1` through `836-motion-live-30` output. Later the dev process exited with code 0, and automation relaunched Electron's default app without project arguments. This mixed sequence is not a controlled close/quit acceptance test. |

### Inconclusive checks and remaining gates

- A title-region drag left measured position `(56, 33)` unchanged. As in the
  earlier runs, the automation path does not qualify native title dragging.
  Keep a human drag check open; no product defect is established by this result.
- The minimize control did not yield a distinguishable minimized state through
  the app-bound observation path. Native minimize/restore remains unaccepted.
- Exact 200% was not established. DevTools' Electron isolated context returned
  `typeof require === "undefined"`; no native zoom API, CSS zoom, device
  emulation or source instrumentation was substituted for the required check.
  This run adds no narrow/high-zoom acceptance evidence.
- System Settings showed VoiceOver on during an attempted walkthrough and off
  after restoration. No audible output or readable caption panel was exposed
  through the available app-bound observations, so no spoken-announcement or
  screen-reader usability result is claimed. No reader-version qualification
  was completed. A human VoiceOver walkthrough remains required.
- The dev launch again detected Node ABI 147 versus Electron ABI 149 for
  `fs-ext`, rebuilt it automatically and verified the module before launch.
  The historical native-build root cause is not established here.
- The host's pre-existing missing `.zshrc` completion-file warning appeared.
  One read-only JavaScript measurement was accidentally sent to the disposable
  Shell during focus routing and produced a shell pattern error; it was rerun
  through a verified DevTools prompt. Neither observation is a product defect.
- Keep #836 open and its broad checkboxes unchanged. Pending work includes
  human title drag, minimize/restore and controlled close/quit, exact 200% and
  narrow layout, complete keyboard/saved-shortcut coverage, spoken screen-reader
  announcements, remaining live attention transitions, and Windows/Linux checks.


## October 23 continuation

This agent-operated run adds saved-shortcut, pinned-inspector input, and native
Quit/restart evidence. It does not complete the broader native chrome, keyboard,
screen-reader or release gates in #836.

### Build and conditions

- Source: `2fec992117c516671dfa8a93a70fafbc3dc6de1d`, including PR #977;
  no runtime source edits or test instrumentation.
- Host: macOS 26.6.2, build 25G83, arm64; Electron 44.6.0; desktop dependencies
  installed with pnpm 11.24.0.
- Owned checkout: `orkworks-shell-validation-836-oct23`; launched from
  `apps/desktop/` with `ORKWORKS_DEV_PORT=5673 pnpm dev`. The renderer URL was
  `http://localhost:5673/`; settings were scoped to this checkout/port.
- Disposable Git workspace: `/private/tmp/orkworks-836-native-20261023`.
  One ordinary Shell session was created. No coding-agent harness was launched
  and no coding-tool integration was enabled.
- Read-only DevTools measurement returned native outer size 1400 × 877,
  position `(56, 33)`, and `devicePixelRatio=2` at 100% on the 2× display.
  DevTools was closed for keyboard/inspector and replay screenshots. Its docked
  viewport dimensions are not breakpoint evidence.
- Native AX and screenshots were inspected in the session, not retained in the
  repository. No screen reader or OS accessibility setting was used or changed.

### Observed results

| Conditions | Action and expected result | Actual result / disposition |
| --- | --- | --- |
| 100%, Settings → Hotkeys | Capture a custom Details chord, save it, and use the saved native command | Detail Panel changed from `CmdOrCtrl+Shift+D` to `CmdOrCtrl+Shift+I`. Capture ended with that value and enabled Save; Save displayed Saved. After closing Settings and dismissing the no-active-tools reminder, the custom chord toggled Details. Closing returned AX focus to the Details button. This is one custom-command sample, not complete shortcut/conflict coverage. |
| 100%, pinned Details beside a live Shell | Escape and Up go to terminal input without closing pinned Details | After pinning Details, focused Terminal input ran `cat -v`. Escape, Up and Return produced visible `^[^[[A` output; pinned Details remained visible and AX retained terminal-input focus. Control+C ended the bounded input check. |
| 100%, pinned Details heading focused | Escape closes the inspector and restores its invoker | Reopened Details through the saved chord; AX focused its heading. Escape removed the inspector and focused Details. This complements terminal-owned Escape above. |
| 100%, live Shell, native Quit | Quit the owned dev app without an automation-triggered relaunch | Command+Q ended the tracked `pnpm dev` process with exit code 0. No subsequent app observation was made until an explicit new dev launch. This qualifies this Quit path, not red-button close, minimize/restore or other platforms. |
| Same source, explicit restart with port 5673 | Saved shortcut persists; ended history remains inspectable without resume | Explicitly reopened the disposable workspace. Initially no session was selected. Selected the existing Shell row, then the custom chord opened Details and focused its heading. Details showed ENDED / Closed with unsupported resume routes; the screenshot retained `836-oct23-before-keyboard` and the `cat -v` output. No new session or resume was requested. |
| After restart inspection | Stop the owned app | Command+Q again ended the tracked dev process with exit code 0. |

### Diagnostics and limits

- Fresh startup again detected the `fs-ext` ABI 147/149 mismatch, rebuilt it
  automatically for Electron 44.6.0, verified it, and launched. The restart
  verified the module without another repair. Existing build warnings do not
  establish a new native-build defect.
- Each collected launch log included a `get-backend-url` handler error,
  `Backend generation has not started`. The log chunk had no timing information
  sufficient to attribute it to startup or shutdown. Workspace connection,
  session/history inspection and Quit still completed. No root cause or fix is
  claimed by this run.
- The host's existing missing `.zshrc` completion-file warning remained visible;
  the Shell marker and input check worked. No host configuration was changed.
- Native folder-picker input needed a coordinate click before typed text took
  effect; this is automation-path evidence, not a demonstrated product defect.
- Keep #836 open and its broad checkboxes unchanged. Exact 200% zoom, human
  title dragging/minimize/restore/red-button close, spoken screen-reader output,
  remaining keyboard/modal/shortcut variants, remaining live attention motion,
  and Windows/Linux qualification are still pending. The successful 100% Quit
  path above does not complete the native chrome checkbox.
