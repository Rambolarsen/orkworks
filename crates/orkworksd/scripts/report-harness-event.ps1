[CmdletBinding()]
param(
    [string]$Marker = ""
)

$observedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.ffffffZ")

$payload = ""
try {
    $payload = [Console]::In.ReadToEnd()
} catch {}

$sessionId = $env:ORKWORKS_SESSION_ID
$port = $env:ORKWORKS_PORT

# Claude Code's hook JSON includes a "cwd" field (its own current working
# directory) on every event, alongside "session_id" below. Forwarding it
# lets the sidecar track where the agent is actually working, not just
# where its process was launched (issue #241).
$reportedCwd = ""
$claudeSessionId = ""
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
                $claudeSessionId = ([string]$data.session_id).Trim()
            }
            if ($data.cwd) {
                $reportedCwd = ([string]$data.cwd).Trim()
            }
        }
    } catch {}
}

# The timeout below bounds the whole request; Invoke-RestMethod has no
# separate fast-fail connect-phase timeout the way curl's --connect-timeout
# does, so a hung connect (not just a slow response) still costs the full
# 5 seconds here.
if ($sessionId -and $port) {
    try {
        $attention = @{ status = "waiting_for_input"; observedAt = $observedAt }
        if ($reportedCwd) {
            $attention["cwd"] = $reportedCwd
        }
        $attentionBody = $attention | ConvertTo-Json -Compress
        Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/attention" `
            -ContentType "application/json" -Body $attentionBody -TimeoutSec 5 | Out-Null
    } catch {}
}

if ($Marker -clike "*:claude-code") {
    if ($sessionId -and $port -and $claudeSessionId) {
        try {
            $sessionBody = @{ harnessSessionId = $claudeSessionId; source = "claude_hook"; confidence = 0.98 } | ConvertTo-Json -Compress
            Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/harness-session" `
                -ContentType "application/json" -Body $sessionBody -TimeoutSec 5 | Out-Null
        } catch {}
    }
}
