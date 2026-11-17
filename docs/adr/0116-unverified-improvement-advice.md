---
type: Architecture Decision
title: Code-owned unverified improvement advice
description: Replace English execution-claim guessing for producer 3 with unverified status and negotiated presentation.
tags: [orkworks, architecture, taskmaster, security]
status: stable
workflow_status: approved
---

# Code-owned unverified improvement advice

- Status: accepted; owner approved the reviewed design on 2026-11-17.
- Decider: repository owner.
- Tracking: [#1694](https://github.com/Rambolarsen/orkworks/issues/1694).
- Contract: [Unverified improvement advice](../superpowers/specs/2026-11-17-unverified-improvement-advice-design.md).
- Supersedes: [ADR 0115](0115-inert-improvement-text-and-fixed-handoffs.md) for the new producer; existing policy-1/2 records retain their validation and bytes.

## Context

Five controlled comparisons exposed repeated harmless-text rejection. A narrow
prospective-validation exception also admitted a real completion claim and was
withdrawn. English classification cannot establish whether checks executed.

## Decision

Producer 3 treats all bounded generated prose as unverified reference data.
Derive `verificationStatus: "unverified"` from consistent producer/report/history
identities in code. Keep structural, citation, privacy and control/bidi validation.
Never infer execution evidence from model text, scores or recommendation status.

Supported consumers declare `X-OrkWorks-Advice-Protocol: 3` through fixed narrow
IPC and show the contract's fixed warning before generated advice. Legacy views
omit new prose and refuse new-record mutations before side effects. Keep fixed
ID-only handoffs with the warning and protocol declaration; refuse overrides and
packets. Exclude new prose from legacy synthesis without removing active IDs.

## Consequences

False completion sentences may be visible as explicitly unverified advice. They
never become verified status or execution authority. Mixed-client/downgrade tests
must cover every public projection and mutation, preserve bytes and avoid false
freshness. No destructive migration or new coding-tool confinement is introduced.
Legacy producers and records remain unchanged. Repository verification and review
are necessary but do not establish usefulness. Activation and another paid trial
remain separately gated.
