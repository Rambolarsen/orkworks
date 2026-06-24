# Ollama Harness Pass-Through Validation

- Date: 2026-06-24
- Status: completed
- Parent: #51

## Harness Compatibility Summary

| Harness | Native Ollama Support | Required Model Format | modelPrefix | Status |
|---------|----------------------|----------------------|-------------|--------|
| Aider | Yes (LiteLLM) | `ollama_chat/<model>` | `ollama_chat/` | Working |
| OpenCode | Yes (provider system) | `ollama/<model>` | `ollama/` | Working |
| Claude Code | No (Anthropic-only) | N/A | — | Not supported |
| Codex | No (OpenAI-only) | N/A | — | Not supported |
| Gemini CLI | No (Gemini-only) | N/A | — | Not supported |

## Implementation

Added `modelPrefix` field to `HarnessConfig`. When set, the prefix is prepended to the model string before substituting into harness args. This transforms a bare Ollama model name (e.g., `llama3.2`) into the format each harness expects:

- Aider: `--model=ollama_chat/llama3.2`
- OpenCode: `--model=ollama/llama3.2`

The `modelPrefix` is always applied when the model is non-empty. For harnesses with `modelPrefix` set, users should select Ollama models from the Peon model picker. Non-Ollama models should be used with harnesses that don't have a prefix set (Claude Code, Codex, Gemini CLI).

## Remaining Gaps

- **Claude Code, Codex, Gemini CLI**: These harnesses are tied to their respective providers and do not accept arbitrary model strings. No code change can enable Ollama pass-through for these harnesses — it would require the CLIs themselves to add Ollama support.
- **Prefix always applied**: The prefix is unconditional when the model is non-empty. A user selecting a non-Ollama model with Aider would get an incorrect prefix. Future work could make the prefix conditional on whether the selected model is from Ollama's model list.
