# Subagent Model Policy

This project defaults to the Luna model tier for delegated implementation and
review work. In the current Codex environment, that tier is exposed as
gpt-5.6-luna; if a requested Luna alias is unavailable, use the closest
available Luna model and state the substitution.

Use Luna for:

- mechanical implementation tasks;
- focused task reviews;
- test-writing and verification tasks; and
- documentation or plan maintenance.

Use a stronger model only when the user explicitly authorizes it or when a
task is blocked after a Luna attempt and the additional reasoning is necessary
to make progress. Any deviation should be visible in the task commentary.

## Revisit note

Re-evaluate this policy whenever a new model family or Luna-tier release is
available, and before starting the next major implementation milestone. Compare
task quality, review findings, turnaround time, and cost, then update the
default model name and exception rules here if the evidence supports a change.

Last reviewed: 2026-08-25.
