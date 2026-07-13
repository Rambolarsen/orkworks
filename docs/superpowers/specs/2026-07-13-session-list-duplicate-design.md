# Session List Duplicate Prevention Design

## Problem

The renderer receives session snapshots from a two-second poll and from the successful create-session request. Those asynchronous responses can overlap. Appending the create response without checking its ID can leave the same session rendered twice.

## Decision

Canonicalize every renderer session snapshot by session ID. When duplicate records occur, retain the most recently supplied record for that ID. Use this operation when accepting the create response and when accepting a poll response, before the existing presentation sort.

This is defensive UI state normalization only. It does not change the sidecar session registry, metadata protocol, session lifecycle, or launch behavior.

## Data flow

```text
poll snapshot -> normalize by session ID -> sort -> sessions state -> list
create response -^
```

## Error handling

Failed API calls retain the current snapshot and continue using the existing silent polling/create error paths. A malformed duplicate is not surfaced as a new error because the renderer can safely represent it once.

## Testing

Add a unit test that starts with a snapshot containing the new session and then incorporates the same create response. It must produce exactly one entry for that ID while retaining distinct sessions and current sort behavior.
