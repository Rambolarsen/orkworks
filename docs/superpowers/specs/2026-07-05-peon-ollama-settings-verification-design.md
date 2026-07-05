# Peon Ollama Settings Verification Design

## Goal

Make Settings show whether the configured Ollama instance is reachable for Peon, and show which Ollama models are reasonable Peon candidates without requiring the user to guess or inspect logs.

## Scope

This design only covers the desktop Settings experience for Peon's Ollama configuration.

It does not add model-by-model inference probing, automatic model selection, or any new non-Ollama provider behavior.

## Current State

The app already persists:

- `providers.ollamaBaseUrl`
- `providers.peonModel`

The sidecar already supports:

- listing Ollama models from `GET /api/tags`
- running Ollama inference through `POST /api/generate`

The current Settings modal only exposes:

- a free-text Ollama base URL input
- a free-text Peon model input with hidden suggestions

The missing behavior is visibility. Users cannot explicitly verify an Ollama instance, cannot see why verification failed, and cannot see a visible list of candidate models for Peon.

## User Outcome

From Settings, the user should be able to:

- test the currently typed Ollama URL before saving it
- see the verification result for the saved URL after saving
- see a visible list of candidate Ollama models Peon can likely use
- pick a Peon model from that visible list or keep typing manually

## Approach Options

### Option 1: Renderer-driven verification against existing model-list endpoint

The renderer would call the existing provider models endpoint with the current draft URL applied temporarily in the renderer only.

Pros:

- minimal backend changes

Cons:

- awkward because the current endpoint reads persisted sidecar settings
- unsaved URL verification becomes indirect and brittle
- error handling and filtering logic would be split awkwardly across renderer and main

### Option 2: Dedicated verification endpoint for Ollama

Add a small sidecar endpoint that accepts a base URL, tests connectivity against Ollama, fetches models, applies a simple Peon-candidate filter, and returns a structured result the Settings UI can render.

Pros:

- directly supports unsaved and saved verification
- keeps Ollama-specific networking in the sidecar
- produces one response shape for connection state, candidate models, and failure reason

Cons:

- adds one focused backend endpoint and response type

### Option 3: Active model probing

Fetch all models, then run a tiny Peon-style test prompt against each and only show models that pass.

Pros:

- strongest compatibility signal

Cons:

- slower, noisier, and more expensive
- too much complexity for the current need

## Decision

Use Option 2.

This keeps the first version simple while still solving both required cases:

- verifying the current unsaved draft URL
- re-verifying the saved URL after persistence

## UX Design

Add an Ollama verification card inside the existing `Model providers` section in Settings.

The card contains:

- the existing `Ollama base URL` input
- a `Verify Ollama` button that tests the current draft URL without saving
- inline verification state
- a visible candidate-model list for Peon
- the existing `Peon model` field, now fed by the visible candidate list

### Verification States

The UI should show one of:

- idle: no verification result yet
- checking: verification in progress
- connected: Ollama reachable and candidate models returned
- connected-empty: Ollama reachable but no Peon candidate models found
- failed: unreachable, timeout, invalid response, or HTTP error

The failure copy should be concrete and backend-supplied when possible, for example:

- `Ollama endpoint unreachable at http://127.0.0.1:11434`
- `Ollama request timed out`
- `Ollama returned HTTP 404`
- `failed to parse Ollama /api/tags response`

### Candidate Model List

The candidate-model list should be a visible list, not only a datalist.

Each row shows:

- model name
- whether it matches the currently selected `Peon model`
- a simple `Use this model` action

The manual text field remains available so the user can still type a model name directly.

### Saved vs Unsaved Verification

`Verify Ollama` always checks the current draft URL from the input.

After saving provider settings successfully, Settings should automatically re-run verification using the saved `ollamaBaseUrl` and refresh the visible candidate list. This keeps the UI aligned with the persisted settings without requiring another manual click.

## Candidate Filter

Keep the first version simple and heuristic-based.

The sidecar should fetch all model names from Ollama, then exclude obvious non-Peon candidates by name.

Initial exclusions:

- names containing `embed`
- names containing `embedding`

Everything else is returned as a Peon candidate.

This is intentionally permissive. It does not guarantee high-quality Peon behavior; it only removes the clearest non-generation models. Stricter validation can be added later if needed.

## API Design

Add a sidecar endpoint dedicated to Settings verification.

### Request

`POST /settings/providers/ollama/verify`

Body:

```json
{
  "baseUrl": "http://127.0.0.1:11434"
}
```

### Response

Success:

```json
{
  "ok": true,
  "baseUrl": "http://127.0.0.1:11434",
  "models": ["llama3.1", "qwen3:8b"],
  "excludedModels": ["nomic-embed-text"],
  "message": "Connected"
}
```

Failure:

```json
{
  "ok": false,
  "baseUrl": "http://127.0.0.1:11434",
  "models": [],
  "excludedModels": [],
  "message": "Ollama endpoint unreachable at http://127.0.0.1:11434"
}
```

Notes:

- `models` means candidate models for Peon after filtering
- `excludedModels` is useful for transparency and debugging in the UI, but can remain visually secondary
- `message` is the user-facing status string

## Architecture

### Sidecar

Add a verification handler under the provider/settings HTTP surface that:

1. normalizes the supplied base URL
2. calls Ollama `/api/tags`
3. parses model names
4. filters obvious non-generation models
5. returns a structured verification payload

Reuse existing Ollama HTTP logic where practical so list-model behavior stays consistent.

### Electron Main Process

Expose one new preload/main-process method for the renderer to request Ollama verification through the sidecar using the active backend port.

No persistent state changes are required for verification itself.

### Renderer

Settings modal state should track:

- current verification status
- returned candidate models
- excluded models
- last verification message

The renderer should:

- verify on explicit button click using the current draft URL
- auto-verify after a successful save of provider settings when `ollamaBaseUrl` changed
- allow selecting a candidate model into `peonModelDraft`

## Error Handling

If verification fails:

- do not clear the draft URL
- do not clear the typed Peon model
- replace the visible candidate list with the failure state
- preserve the last successful save status independently from verification status

If Ollama is reachable but no candidate models remain after filtering:

- show a successful connection state
- show that no Peon candidate models were found
- keep manual model entry available

## Testing

Add coverage for:

- backend verification success
- backend verification failure on unreachable endpoint
- backend filtering of obvious embedding models
- renderer Settings markup/state for verify action and visible candidate-model list
- automatic re-verification after saving a changed Ollama base URL

## Out of Scope

The following are intentionally excluded from this design:

- probing every model with a live Peon test prompt
- ranking models by quality
- validating that the selected model produces good Peon inference
- auto-selecting a model after verification
- expanding similar verification UX to non-Ollama providers
