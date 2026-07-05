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

The renderer-owned failure copy should be concrete and derived from structured response fields, for example:

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

After saving provider settings successfully, Settings should automatically re-run verification only when the saved `ollamaBaseUrl` changed. Saving only `peonModel` does not trigger re-verification; the last verification result remains visible and should indicate which normalized URL it applies to. This keeps the UI aligned with the persisted settings without requiring another manual click while avoiding unnecessary duplicate requests on model-only edits.

## Candidate Filter

Keep the first version simple and heuristic-based.

The sidecar should fetch all model names from Ollama, then exclude obvious non-Peon candidates by name.

Initial exclusions:

- case-insensitive names containing `embed`
- case-insensitive names containing `embedding`

Everything else is returned as a Peon candidate.

Matching should be applied against the full Ollama model name returned by `/api/tags`, including tag suffixes such as `:latest`.

This is intentionally permissive. It does not guarantee high-quality Peon behavior; it only removes the clearest non-generation models. Known false negatives remain possible, for example embedding-style model names that do not include `embed` or `embedding`. Stricter validation can be added later if needed.

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

Route note:

- the renderer reaches this through Electron main using `http://127.0.0.1:{port}/settings/providers/ollama/verify`
- this follows the existing sidecar settings namespace, alongside `POST /settings/providers`

### Input Validation and Normalization

The verify endpoint accepts the same URL shape as persisted `providers.ollamaBaseUrl`.

Rules:

- required, non-empty string after trim
- accepted schemes: `http://` and `https://` only
- trailing slashes removed before use or comparison
- path/query/fragment components are rejected; the base URL must be origin-only
- arbitrary hosts are allowed; this is not restricted to localhost
- invalid input returns a validation failure without making an outbound request

The sidecar should return the normalized URL in every response so the renderer can label the result consistently and ignore stale results for older draft values.

Network constraints:

- request timeout: 10 seconds
- response parsing should only read the expected `/api/tags` JSON body
- transport, timeout, HTTP, and parse failures are distinct failure cases from an empty model list

### Response Shape

The response should use structured fields for UI state and diagnostics. Renderer copy should be derived from these fields rather than relying on backend-written user-facing prose.

Fields:

- `ok`: whether the request reached Ollama and the `/api/tags` response was parsed successfully
- `normalizedBaseUrl`: normalized URL the result applies to
- `status`: one of `connected`, `connected_empty`, or `failed`
- `reasonCode`: machine-readable failure or empty-result reason
- `httpStatus`: optional HTTP status code for non-2xx responses
- `models`: candidate models for Peon after filtering
- `excludedModels`: models excluded by the heuristic filter
- `diagnostic`: optional backend diagnostic detail for logs or secondary UI text

Reachable-but-empty must be represented as success. That includes:

- Ollama returned zero models
- Ollama returned only models that were filtered out

Both cases return `ok: true` with `models: []`, not a transport failure.

### Response Examples

Success:

```json
{
  "ok": true,
  "normalizedBaseUrl": "http://127.0.0.1:11434",
  "status": "connected",
  "reasonCode": "connected",
  "httpStatus": 200,
  "models": ["llama3.1", "qwen3:8b"],
  "excludedModels": ["nomic-embed-text"],
  "diagnostic": null
}
```

Reachable but no candidates:

```json
{
  "ok": true,
  "normalizedBaseUrl": "http://127.0.0.1:11434",
  "status": "connected_empty",
  "reasonCode": "all_models_filtered",
  "httpStatus": 200,
  "models": [],
  "excludedModels": ["nomic-embed-text"],
  "diagnostic": null
}
```

Failure:

```json
{
  "ok": false,
  "normalizedBaseUrl": "http://127.0.0.1:11434",
  "status": "failed",
  "reasonCode": "unreachable",
  "httpStatus": null,
  "models": [],
  "excludedModels": [],
  "diagnostic": "Ollama endpoint unreachable at http://127.0.0.1:11434"
}
```

Notes:

- `models` means candidate models for Peon after filtering
- `excludedModels` is useful for transparency and debugging in the UI, but can remain visually secondary
- `diagnostic` is not primary UI copy; renderer-owned text should be derived from `status`, `reasonCode`, and `httpStatus`

## Architecture

### Sidecar

Add a verification handler under the provider/settings HTTP surface that:

1. normalizes the supplied base URL
2. calls Ollama `/api/tags`
3. parses model names
4. filters obvious non-generation models
5. returns a structured verification payload

Reuse existing Ollama HTTP logic where practical so list-model behavior stays consistent.

The verify handler should not reuse the existing `GET /providers/:id/models` empty-list behavior directly, because verification needs to distinguish:

- unreachable or invalid Ollama endpoint
- reachable Ollama with zero models
- reachable Ollama with models that all filtered out

Only the first case is a failed verification.

### Electron Main Process

Expose one new preload/main-process method for the renderer to request Ollama verification through the sidecar using the active backend port.

No persistent state changes are required for verification itself.

Verification results bypass the existing `getProviderModels("ollama")` cache. They are ephemeral Settings-state results tied to a specific normalized URL, not a replacement for the general provider model cache. Saving provider settings should continue invalidating the cached `ollama` provider-model entry as it does today.

### Renderer

Settings modal state should track:

- current verification status
- normalized URL the current result applies to
- returned candidate models
- excluded models
- last verification reason code / diagnostic

The renderer should apply a verification result only if it still matches the current request context. At minimum, the result's `normalizedBaseUrl` must match the draft URL the request was issued for. A slower stale verification result must not overwrite a newer one after the user edits the URL or after a later save-triggered verification completes.

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

If the currently typed or saved `peonModel` is not present in the candidate list:

- keep the manual value intact
- show it as the current selection
- do not silently clear or replace it

## Testing

Add coverage for:

- backend verification success
- backend verification failure on unreachable endpoint
- backend validation failure on empty or invalid URL
- backend timeout, HTTP non-2xx, and parse failure cases
- backend reachable-empty result when Ollama returns zero models
- backend reachable-empty result when all returned models are filtered out
- backend filtering of obvious embedding models
- backend normalization of trailing slash handling
- renderer Settings markup/state for verify action and visible candidate-model list
- automatic re-verification after saving a changed Ollama base URL
- renderer stale-result protection when multiple verification requests overlap
- renderer handling for a current selected model that is absent from the candidate list

## UI Constraints

The candidate-model list should:

- sort models alphabetically
- highlight the currently selected model
- remain scrollable within a bounded container instead of expanding the modal indefinitely
- expose an empty state for `connected_empty`

Accessibility requirements:

- `Verify Ollama` is disabled while verification is `checking`
- verification state is announced through an accessible status region
- candidate rows and `Use this model` actions have stable accessible labels suitable for renderer tests

## Out of Scope

The following are intentionally excluded from this design:

- probing every model with a live Peon test prompt
- ranking models by quality
- validating that the selected model produces good Peon inference
- auto-selecting a model after verification
- expanding similar verification UX to non-Ollama providers
