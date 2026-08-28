# Copilot Native Voice Capability Design

## Context

GitHub Copilot CLI exposes a `/voice` command for harness-owned dictation via
Foundry Local. OrkWorks currently has the generic voice capability model and
plumbing, but the built-in Copilot definition does not declare that capability
and the desktop session presentation does not visibly surface it.

## Decision

Declare native voice support on the built-in `copilot` harness definition:

```json
"voice": {
  "nativeVoice": true,
  "requiresMicrophonePermission": true,
  "orkworksDictation": false,
  "orkworksVoiceCommands": false
}
```

Use the existing resolved harness capability data in the desktop UI to show a
compact native-voice indicator for sessions whose harness advertises it. The
indicator will identify native voice support and microphone permission as not
verified; OrkWorks will not attempt to access the microphone or infer runtime
permission from the PTY.

## Boundaries

- No audio capture, recording, proxying, storage, or PTY transport is added.
- No OS permission API or packaging change is added in this issue.
- Copilot remains responsible for its `/voice` behavior and Foundry Local.
- Existing harness capability serialization and session APIs are reused.

## Testing

- Sidecar registry/definition tests prove Copilot resolves the voice capability
  and advertises the voice capability name.
- Desktop presentation tests prove a native-voice capability renders the
  indicator and a voice-less harness does not.
- Existing Rust and desktop test suites remain green.
