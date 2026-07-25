[CmdletBinding()]
param(
    [string]$Marker = ""
)

$ErrorActionPreference = "SilentlyContinue"

$observedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.ffffffZ")

$payload = ""
try {
    $payload = [Console]::In.ReadToEnd()
} catch {
    $payload = ""
}

$sessionId = $env:ORKWORKS_SESSION_ID
$port = $env:ORKWORKS_PORT

if ($sessionId -and $port) {
    try {
        $attentionBody = @{ status = "waiting_for_input"; observedAt = $observedAt } | ConvertTo-Json -Compress
        Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/attention" `
            -ContentType "application/json" -Body $attentionBody -TimeoutSec 5 | Out-Null
    } catch {}
}

if ($Marker -like "*:claude-code") {
    $claudeSessionId = ""
    try {
        $data = $payload | ConvertFrom-Json
        if ($data.session_id) {
            $claudeSessionId = [string]$data.session_id
        }
    } catch {}

    if ($sessionId -and $port -and $claudeSessionId) {
        try {
            $sessionBody = @{ harnessSessionId = $claudeSessionId; source = "claude_hook"; confidence = 0.98 } | ConvertTo-Json -Compress
            Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$port/sessions/$sessionId/harness-session" `
                -ContentType "application/json" -Body $sessionBody -TimeoutSec 5 | Out-Null
        } catch {}
    }
}
