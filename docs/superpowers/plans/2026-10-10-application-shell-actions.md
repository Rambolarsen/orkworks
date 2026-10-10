---
type: "Implementation Plan"
title: "Delivery plan: ordinary Actions overview"
description: "Implementation plan: Delivery plan: ordinary Actions overview."
tags: ["orkworks", "plans"]
---

# Delivery plan: ordinary Actions overview

**Issue:** [#805](https://github.com/Rambolarsen/orkworks/issues/805)  
**Depends on:** [#779](https://github.com/Rambolarsen/orkworks/issues/779), the shared shell

[Back to the delivery map](2026-10-08-application-shell-redesign.md)

## What changes for the user

The user can open one read-only overview that answers, “What should I look at
now?” It shows ordinary session attention first and existing recommendations
afterward. The two sources have separate counts. “Show next action” takes the
user to the first available item in that order.

The overview is not a command center: looking at an item does not select its
session, clear attention, send a prompt, approve a proposal, or run work.

## How to tell it works

- The overview shows current attention and recommendations in their approved
  order, with separate counts.
- Counts remain visible when other shell areas are hidden. Missing or partial
  source data is explained instead of shown as zero.
- “Show next action” reveals and focuses the first available item without
  changing session selection or terminal focus.
- Opening an item and returning leaves the user at the same valid place, with
  focus restored when possible.
- The same behavior is easy to use from the wide inspector and from medium and
  compact temporary pages.

Use #805 for exact source ordering, unavailable-item rules, and automated
checks. This piece uses the destination host from #779 and does not depend on
Review (#780). Workflow decisions and unviewed run results remain gated by
#610 and #741/#743/#744/#746; Capacity stays separate.

## Complexity and review

| Dimension | Rating | Evidence |
| --- | --- | --- |
| Dependencies | 3 | Needs the shared responsive destinations from #779 and reads two existing sources. |
| Blast radius | 2 | This is a read-only overview; its main risk is misleading counts or accidentally changing attention. |
| State changes | 1 | It adds no authority or durable user state. |
| Reversibility | 1 | The overview can be removed without changing session or recommendation records. |
| Uncertainty | 3 | Real source availability and stale targets must be verified through integration tests. |

**Total: 10/25. Plan quality: Ready.** The scope is read-only and the issue
defines how missing data and navigation should behave.
