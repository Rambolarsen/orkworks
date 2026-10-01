[CmdletBinding()]
param(
    [string]$Marker = "",
    [string]$Status = "waiting_for_input",
    [string]$HookFingerprint = "",
    [string]$Event = "",
    # Plan-path mode (ADR 0038): set by Claude's installed PostToolUse
    # Write|Edit hook entry. In this mode the reporter forwards the hook
    # payload's `tool_input.file_path` to `/sessions/:id/plan-path` and
    # skips the generic attention + harness-session POSTs entirely;
    # the sidecar canonicalizes/rejects the path.
    [switch]$ReportPlanPath
)

$payload = ""
try {
    $payload = [Console]::In.ReadToEnd()
} catch {}

$sessionId = $env:ORKWORKS_SESSION_ID
$port = $env:ORKWORKS_PORT

# Plan-path mode: extract `tool_input.file_path`, apply a cheap lexical
# whitelist (Markdown; one of the recognized plan/spec roots), POST once to
# /sessions/:id/plan-path, and exit. The cheap filter is never the authority
# — `report_session_plan_path` canonicalizes and rejects non-Markdown,
# workspace-escaping, symlink-pivoting, and missing files. The filter exists
# only to keep unrelated Write/Edit calls from spamming the route.
if ($ReportPlanPath) {
    if ($sessionId -and $port) {
        try {
            $data = $payload | ConvertFrom-Json
            if ($data -is [System.Management.Automation.PSCustomObject]) {
                $file = ""
                if ($data.tool_input -and $data.tool_input.file_path) {
                    $file = ([string]$data.tool_input.file_path).Trim()
                }
                if ($file) {
                    $isMarkdown = $file -like "*.md"
                    $inSpecsDir = $file -like "*\specs\*" -or $file -like "*/specs/*"
                    $inPlansDir = $file -like "*\docs\superpowers\plans\*" -or $file -like "*/docs/superpowers/plans/*"
                    $inSuperpowerSpecs = $file -like "*\docs\superpowers\specs\*" -or $file -like "*/docs/superpowers/specs/*"
                    if ($isMarkdown -and ($inSpecsDir -or $inPlansDir -or $inSuperpowerSpecs)) {
                        $body = @{ planPath = $file } | ConvertTo-Json -Compress
                        Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/plan-path" `
                            -ContentType "application/json" -Body $body -TimeoutSec 5 | Out-Null
                    }
                }
            }
        } catch {}
    }
    exit
}

# Claude Code's hook JSON includes a "cwd" field (its own current working
# directory) on every event, alongside "session_id" below. Forwarding it
# lets the sidecar track where the agent is actually working, not just
# where its process was launched (issue #241). Codex's SessionStart payload
# carries other fields too (cwd, hook_event_name, source, ...). The native ID
# may be recovered from any owned Codex hook; SessionStart alone supplies
# lifecycle metadata used by the replacement guard. Copilot's notification payload uses camelCase
# "sessionId" (not "session_id") per
# https://docs.github.com/en/copilot/reference/hooks-reference, alongside its
# own "cwd". $sessionSource doubles as the harness-session
# "source" field below and as the marker for "this event isn't a needs-input
# signal" further down — one extraction point instead of matching $Marker a
# second and third time.
$reportedCwd = ""
$harnessSessionId = ""
$sessionStartSource = ""
$sessionStartEvent = ""
$sessionSource = ""
$codexAttention = $false
$codexCaptureOnly = $false
$codexPayloadCapture = $null
$safeCodexPayloadKeys = @("hook_event_name", "model", "permission_mode", "turn_id", "tool_name", "tool_response")
$attentionPostKind = "not_applicable"
$harnessSessionPostKind = "skipped_no_harness_session_id"

if ($Marker -clike "*:claude-code") {
    try {
        $data = $payload | ConvertFrom-Json
        # $data must be a single object, not an array: PowerShell's
        # member-enumeration-over-collections would otherwise return a
        # per-element array of $nulls for `.session_id`/`.cwd` on an array
        # payload, which is truthy for 2+ elements and stringifies to a
        # non-empty, space-joined garbage value instead of failing safely.
        if ($data -is [System.Management.Automation.PSCustomObject]) {
            if ($data.session_id) {
                $harnessSessionId = ([string]$data.session_id).Trim()
            }
            if ($data.cwd) {
                $reportedCwd = ([string]$data.cwd).Trim()
            }
        }
    } catch {}
    $sessionSource = "claude_hook"
} elseif ($Marker -clike "*:codex") {
    try {
        $data = $payload | ConvertFrom-Json
        if ($data -is [System.Management.Automation.PSCustomObject] -and $data.session_id -is [string] -and $data.session_id) {
            $harnessSessionId = ([string]$data.session_id).Trim()
        }
        if ($data -is [System.Management.Automation.PSCustomObject] -and $Event -in @("PermissionRequest", "PostToolUse")) {
            $payloadKeys = @(
                $data.PSObject.Properties.Name | Where-Object {
                    $_ -in $safeCodexPayloadKeys
                } | Sort-Object -Unique | Select-Object -First 64
            )
            $payloadScalars = @{}
            foreach ($key in @("hook_event_name", "permission_mode", "turn_id", "tool_name")) {
                $value = $data.$key
                if ($null -ne $value -and $value -is [string] -and $value.Length -le 128) {
                    $payloadScalars[$key] = $value
                } elseif ($null -ne $value -and $value -is [ValueType]) {
                    $payloadScalars[$key] = $value
                }
            }
            $codexPayloadCapture = @{ payloadKeys = $payloadKeys; payloadScalars = $payloadScalars }
        }
        if ($Event -eq "SessionStart" -and $data -is [System.Management.Automation.PSCustomObject] -and $data.source -in @("startup", "resume", "clear", "compact")) {
            $sessionStartSource = [string]$data.source
            $sessionStartEvent = "SessionStart"
        }
    } catch {}
    $sessionSource = "codex_hook"
    if ($Event -eq "PostToolUse") {
        $codexCaptureOnly = $true
        $attentionPostKind = "skipped_capture_only"
        $harnessSessionPostKind = "skipped_capture_only"
    }
    switch ($Event) {
        "UserPromptSubmit" {
            $Status = "working"
            $codexAttention = $true
        }
        "PermissionRequest" {
            $Status = "waiting_for_input"
            $codexAttention = $true
        }
        "Stop" {
            $Status = "idle"
            $codexAttention = $true
        }
    }
} elseif ($Marker -clike "*:copilot") {
    try {
        $data = $payload | ConvertFrom-Json
        if ($data -is [System.Management.Automation.PSCustomObject]) {
            if ($data.sessionId) {
                $harnessSessionId = ([string]$data.sessionId).Trim()
            }
            if ($data.cwd) {
                $reportedCwd = ([string]$data.cwd).Trim()
            }
        }
    } catch {}
    $sessionSource = "copilot_hook"
}

# The timeout below bounds the whole request; Invoke-RestMethod has no
# separate fast-fail connect-phase timeout the way curl's --connect-timeout
# does, so a hung connect (not just a slow response) still costs the full
# 5 seconds here.
#
# Codex's SessionStart event captures identity only. Turn events carry their
# explicit normalized status and provenance so the sidecar can validate the
# deterministic signal without trusting mutable payload text.
if (-not $codexCaptureOnly -and $sessionId -and $port -and ($sessionSource -ne "codex_hook" -or $codexAttention)) {
    try {
        $observedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.ffffffZ")
        $attention = @{ status = $Status; observedAt = $observedAt }
        if ($reportedCwd) {
            $attention["cwd"] = $reportedCwd
        }
        if ($sessionSource -eq "codex_hook") {
            $attention["source"] = $sessionSource
            $attention["event"] = $Event
            if ($HookFingerprint) {
                $attention["hookFingerprint"] = $HookFingerprint
            }
        }
        $attentionBody = $attention | ConvertTo-Json -Compress
        Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/attention" `
            -ContentType "application/json" -Body $attentionBody -TimeoutSec 5 | Out-Null
        $attentionPostKind = "posted"
    } catch {
        $attentionPostKind = "failed"
    }
} elseif ($sessionSource -eq "codex_hook" -and $codexAttention) {
    $attentionPostKind = "skipped_missing_environment"
}

if (-not $codexCaptureOnly -and $sessionId -and $port -and $harnessSessionId -and $sessionSource) {
    try {
        $sessionReport = @{ harnessSessionId = $harnessSessionId; source = $sessionSource; confidence = 0.98 }
        if ($sessionSource -eq "codex_hook" -and $HookFingerprint) {
            $sessionReport["hookFingerprint"] = $HookFingerprint
        }
        if ($sessionSource -eq "codex_hook" -and $sessionStartSource) {
            $sessionReport["sessionStartSource"] = $sessionStartSource
            $sessionReport["sessionStartEvent"] = $sessionStartEvent
        }
        $sessionBody = $sessionReport | ConvertTo-Json -Compress
        $reportSpooled = $false
        if ($sessionSource -eq "codex_hook" -and $env:ORKWORKS_CODEX_SESSION_REPORT_DIR) {
            $temporaryReport = $null
            try {
                $envelopeBody = @{ report = $sessionReport } | ConvertTo-Json -Compress -Depth 8
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($envelopeBody)
                if ($bytes.Length -le 4096) {
                    $directory = $env:ORKWORKS_CODEX_SESSION_REPORT_DIR
                    $temporaryReport = Join-Path $directory (".pending-" + [guid]::NewGuid().ToString("N"))
                    $publishedReport = Join-Path $directory ([guid]::NewGuid().ToString("N") + ".json")
                    [System.IO.File]::WriteAllBytes($temporaryReport, $bytes)
                    [System.IO.File]::Move($temporaryReport, $publishedReport)
                    $reportSpooled = $true
                }
            } catch {
                if ($temporaryReport -and [System.IO.File]::Exists($temporaryReport)) {
                    try { [System.IO.File]::Delete($temporaryReport) } catch {}
                }
            }
        }
        if (-not $reportSpooled) {
            $sessionHeaders = @{}
            if ($env:ORKWORKS_REPORT_TOKEN) {
                $sessionHeaders["Authorization"] = "Bearer $($env:ORKWORKS_REPORT_TOKEN)"
            }
            Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/harness-session" `
                -Headers $sessionHeaders -ContentType "application/json" -Body $sessionBody -TimeoutSec 5 | Out-Null
        }
        $harnessSessionPostKind = "posted"
    } catch {
        $harnessSessionPostKind = "failed"
    }
} elseif ($sessionSource -eq "codex_hook" -and -not $codexCaptureOnly) {
    if (-not $harnessSessionId) {
        $harnessSessionPostKind = "skipped_no_harness_session_id"
    } else {
        $harnessSessionPostKind = "skipped_missing_environment"
    }
}

# Keep the same private redacted local diagnostic as the POSIX reporter. The
# capture-only exception stores an allowlist of top-level key names plus only
# hook_event_name, permission_mode, turn_id, and tool_name scalar values.
# Sensitive key names and values, paths, session IDs, tokens, payloads, and
# arbitrary free text are excluded.
if ($sessionSource -eq "codex_hook" -and $HOME) {
    $diagnosticDirectory = Join-Path $HOME ".orkworks/hook-scripts"
    $diagnosticPath = Join-Path $diagnosticDirectory "report-harness-event-diagnostic.json"
    $diagnosticMutex = $null
    $temporaryDiagnostic = $null
    function Set-PrivateDiagnosticAcl {
        param([string]$Path)
        if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
            $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
            $acl = New-Object System.Security.AccessControl.FileSecurity
            $acl.SetAccessRuleProtection($true, $false)
            $acl.SetOwner($identity.User)
            $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
                $identity.User,
                [System.Security.AccessControl.FileSystemRights]::FullControl,
                [System.Security.AccessControl.AccessControlType]::Allow
            )
            $acl.SetAccessRule($rule)
            Set-Acl -LiteralPath $Path -AclObject $acl
        }
    }
    function Get-PrivateDiagnosticResult {
        param($Value)
        $result = $Value.result
        if ($result -in @("posted", "failed", "enqueued", "skipped_no_harness_session_id", "skipped_missing_environment", "skipped_capture_only", "not_applicable")) {
            return @{ result = $result }
        }
        return @{ result = "not_applicable" }
    }
    try {
        [System.IO.Directory]::CreateDirectory($diagnosticDirectory) | Out-Null
        # A named mutex serializes the read/merge/replace transaction without
        # leaving a lock file whose ACL or inode lifecycle must be managed.
        $diagnosticMutex = [System.Threading.Mutex]::new($false, "Local\OrkWorks.ReportHarnessEventDiagnostic")
        try {
            $null = $diagnosticMutex.WaitOne()
        } catch [System.Threading.AbandonedMutexException] {
            # WaitOne throws only after granting ownership of an abandoned mutex.
        }
        $captures = @{}
        if ([System.IO.File]::Exists($diagnosticPath)) {
            try {
                $previous = [System.IO.File]::ReadAllText($diagnosticPath) | ConvertFrom-Json
                $oldCaptures = $previous.codexPayloadCapture
                foreach ($oldEvent in @("PermissionRequest", "PostToolUse")) {
                    $old = $oldCaptures.$oldEvent
                    if ($old) {
                        $oldKeys = @($old.payloadKeys | Where-Object {
                            $_ -is [string] -and $_ -in $safeCodexPayloadKeys
                        } | Sort-Object -Unique | Select-Object -First 64)
                        $oldScalars = @{}
                        foreach ($key in @("hook_event_name", "permission_mode", "turn_id", "tool_name")) {
                            $value = $old.payloadScalars.$key
                            if ($null -ne $value -and $value -is [string] -and $value.Length -le 128) {
                                $oldScalars[$key] = $value
                            } elseif ($null -ne $value -and $value -is [ValueType]) {
                                $oldScalars[$key] = $value
                            }
                        }
                        $captures[$oldEvent] = @{
                            payloadKeys = $oldKeys
                            payloadScalars = $oldScalars
                            attentionPost = Get-PrivateDiagnosticResult $old.attentionPost
                            harnessSessionPost = Get-PrivateDiagnosticResult $old.harnessSessionPost
                        }
                    }
                }
            } catch {}
        }
        if ($Event -in @("PermissionRequest", "PostToolUse") -and $null -ne $codexPayloadCapture) {
            $codexPayloadCapture["attentionPost"] = @{ result = $attentionPostKind }
            $codexPayloadCapture["harnessSessionPost"] = @{ result = $harnessSessionPostKind }
            $captures[$Event] = $codexPayloadCapture
        }
        $record = @{
            event = $(if ($Event -in @("SessionStart", "UserPromptSubmit", "PermissionRequest", "PostToolUse", "Stop")) { $Event } else { "Unknown" })
            harnessSessionIdParsed = [bool]$harnessSessionId
            orkworksSessionIdPresent = [bool]$sessionId
            portPresent = [bool]$port
            reportTokenPresent = [bool]$env:ORKWORKS_REPORT_TOKEN
            attentionPost = @{ result = $attentionPostKind }
            harnessSessionPost = @{ result = $harnessSessionPostKind }
            codexPayloadCapture = $captures
        }
        $diagnosticBody = $record | ConvertTo-Json -Compress -Depth 8
        $temporaryDiagnostic = Join-Path $diagnosticDirectory (".report-harness-event-" + [guid]::NewGuid().ToString("N"))
        [System.IO.File]::WriteAllText($temporaryDiagnostic, $diagnosticBody, [System.Text.UTF8Encoding]::new($false))
        Set-PrivateDiagnosticAcl $temporaryDiagnostic

        if ([System.IO.File]::Exists($diagnosticPath)) {
            Set-PrivateDiagnosticAcl $diagnosticPath
            [System.IO.File]::Replace($temporaryDiagnostic, $diagnosticPath, $null)
        } else {
            [System.IO.File]::Move($temporaryDiagnostic, $diagnosticPath)
        }
        Set-PrivateDiagnosticAcl $diagnosticPath
        $temporaryDiagnostic = $null
    } catch {
        if ($temporaryDiagnostic -and [System.IO.File]::Exists($temporaryDiagnostic)) {
            try { [System.IO.File]::Delete($temporaryDiagnostic) } catch {}
        }
    } finally {
        if ($diagnosticMutex) {
            $diagnosticMutex.ReleaseMutex()
            $diagnosticMutex.Dispose()
        }
    }
}
