# Native Harness Voice Support

## Purpose

Some AI terminal harnesses may support voice input directly. OrkWorks should support this by making sure the harness process can access the operating system microphone, while keeping the voice feature owned by the harness itself.

Native harness voice is not OrkWorks dictation.

```text
Native harness voice = the harness uses the OS microphone directly
OrkWorks dictation = OrkWorks captures speech, transcribes it, and sends text to a terminal
```

For the MVP, native harness voice should be handled as a pass-through capability.

## Product Boundary

OrkWorks hosts and supervises AI terminal sessions. It should not become a voice proxy, audio router, or replacement for harness-native voice features.

When a harness supports voice, OrkWorks should:

- display that the current session is voice-capable
- launch the harness process in an environment where OS microphone access can work
- surface microphone permission problems in the UI
- avoid intercepting, recording, storing, or forwarding audio by default
- treat native voice as a harness capability, not as an OrkWorks-owned feature

OrkWorks should not:

- capture microphone audio for native harness voice
- proxy audio through xterm.js or the PTY
- store voice recordings
- auto-send voice-derived commands to terminals
- override the harness' own voice behavior

## Important Technical Constraint

A terminal PTY only handles text input and output.

Native voice does not flow through this path:

```text
xterm.js -> PTY -> microphone audio
```

Instead, the harness process must access the microphone through the operating system:

```text
Harness process -> OS microphone API
```

Because OrkWorks launches the harness process, microphone permission may apply to the OrkWorks desktop app, the Rust sidecar, or the spawned child process depending on the operating system.

## Runtime Model

Recommended process structure:

```text
OrkWorks.app
└── orkworksd
    └── harness process
        └── OS microphone API
```

The terminal remains responsible for text input/output:

```text
OrkWorks frontend
└── xterm.js
    └── WebSocket
        └── orkworksd
            └── PTY
                └── harness stdin/stdout/stderr
```

Voice remains separate:

```text
harness process
└── OS microphone API
```

## Harness Capability Configuration

Harnesses should be able to declare native voice support in configuration.

```json
{
  "version": 2,
  "overrides": {
    "example-harness": {
      "voice": {
        "nativeVoice": true,
        "requiresMicrophonePermission": true,
        "orkworksDictation": false,
        "orkworksVoiceCommands": false
      }
    }
  }
}
```

The built-in definition owns its immutable ID, launch command, and other
capabilities. This sparse override changes only voice metadata; omitted fields
retain the built-in value. Voice remains metadata-only and cannot configure
audio handling or integration commands.

Suggested capability fields:

| Field | Meaning |
| --- | --- |
| `nativeVoice` | The harness has its own voice mode. |
| `requiresMicrophonePermission` | The harness needs OS microphone access. |
| `orkworksDictation` | OrkWorks may offer its own dictation layer for this harness. Separate from native voice. |
| `orkworksVoiceCommands` | OrkWorks may interpret voice commands for OrkWorks itself. Separate from native voice. |

## Session Metadata

Session metadata may include voice capability and runtime voice status.

```json
{
  "id": "upload-refactor",
  "harness": "example-harness",
  "model": "example-model",
  "status": "working",
  "voice": {
    "nativeVoiceSupported": true,
    "mode": "native_harness",
    "microphonePermission": "unknown",
    "lastChecked": "2026-06-15T13:30:00+02:00"
  }
}
```

Suggested microphone permission values:

- `unknown`
- `allowed`
- `denied`
- `not_required`
- `unsupported`
- `error`

## UI Behavior

The active terminal header should show voice capability when relevant.

Example when voice is supported and permission appears available:

```text
Claude Code / Sonnet
Voice: native supported · mic allowed
```

Example when permission is unknown:

```text
Claude Code / Sonnet
Voice: native supported · mic not verified
```

Example when permission is blocked:

```text
Claude Code / Sonnet
Voice: native supported · mic blocked

[Open microphone settings]
```

Example when the harness does not support voice:

```text
OpenCode / DeepSeek
Voice: not supported by harness
```

## Microphone Health Check

OrkWorks should provide a lightweight voice readiness check per session or harness.

The check should answer:

```text
Does this harness declare native voice support?
Does this OS require app-level microphone permission?
Does OrkWorks appear to have microphone permission?
Can the child process likely access the microphone?
```

The MVP does not need to prove that the harness can successfully use voice. It is enough to surface likely permission issues and give the user a clear next action.

## Platform Notes

### macOS

The Electron app may need a microphone usage description in `Info.plist`:

```xml
<key>NSMicrophoneUsageDescription</key>
<string>Allow AI terminal harnesses launched by OrkWorks to use voice input.</string>
```

Depending on packaging and process structure, microphone permission may be associated with the app bundle that launches the sidecar and child process.

### Windows

The user may need to allow microphone access for desktop apps in Windows privacy settings.

OrkWorks should show a clear hint when microphone access appears blocked.

### Linux

Voice access depends on the desktop environment, PipeWire/PulseAudio configuration, sandboxing, and package format.

If OrkWorks is packaged as Flatpak, Snap, or another sandboxed format, explicit microphone permissions may be required.

## Native Voice vs OrkWorks Voice

Native harness voice and OrkWorks voice should remain separate concepts.

| Feature | Owner | Audio captured by OrkWorks? | Sends text to terminal? |
| --- | --- | --- | --- |
| Native harness voice | Harness | No | No, unless the harness itself does it |
| OrkWorks dictation | OrkWorks | Yes | Only after user confirmation |
| OrkWorks voice commands | OrkWorks | Yes | Not by default; acts on OrkWorks UI/actions |

For MVP native voice support, OrkWorks should only support the first row.

## MVP Scope

### Must Have

- Add native voice capability fields to harness configuration.
- Show native voice support in the active session UI.
- Add microphone permission/status field to session metadata.
- Add platform-specific documentation for microphone permissions.
- Do not capture, proxy, or store audio for native harness voice.

### Should Have

- Add a microphone readiness indicator.
- Add an “Open microphone settings” action where practical.
- Add session event log entries when microphone permission problems are detected.
- Allow users to manually mark voice as working or blocked for a harness.

### Could Have Later

- Harness-specific voice detection.
- Automated child-process microphone probe.
- OrkWorks dictation mode.
- OrkWorks voice commands.
- Push-to-talk overlay.
- Voice transcript drafts.

## Event Log Examples

When a session starts with native voice support:

```json
{
  "time": "2026-06-15T13:30:00+02:00",
  "type": "note",
  "summary": "Harness declares native voice support. Microphone permission is unknown."
}
```

When microphone access appears blocked:

```json
{
  "time": "2026-06-15T13:32:00+02:00",
  "type": "blocked",
  "summary": "Native voice may not work because microphone permission appears to be denied."
}
```

## Design Principle

OrkWorks should make native harness voice visible and debuggable without owning the voice interaction.

The harness owns voice.

OrkWorks owns session visibility, process hosting, permissions awareness, and user-facing status.
