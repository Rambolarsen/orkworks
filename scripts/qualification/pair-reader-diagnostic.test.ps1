# Loaded only by the guards invocation; no child processes or external inputs.
function Invoke-DiagnosticGuards($Reader) {
    $cases=[ordered]@{
        traversal='format-denied';absolute='format-denied';backslash='format-denied'
        ads='format-denied';empty='format-denied';dot='format-denied'
        'double-slash'='format-denied';nul='format-denied';'trailing-dot'='format-denied'
        reparse='input-denied';'path-limit'='format-denied';'foreign-root'='input-denied'
        unc='input-denied';'file-limit'='input-limit';'short-stream'='identity-drift'
        'hash-limit'='hash-limit';growth='identity-drift';'nested-lease'='fixture-assertion'
        'encoded-limit'='output-limit';'wrong-path-identity'='identity-drift'
        duplicate='input-limit';'case-fold'='input-limit';type='format-denied'
        'tar-traversal'='format-denied';extra='format-denied';content='identity-drift'
        missing='format-denied';'wrong-archive'='identity-drift';'member-limit'='input-limit'
        'expanded-limit'='input-limit';'extra-file'='format-denied';helper='format-denied'
    }
    $results=[Collections.Generic.List[string]]::new()
    foreach($name in $cases.Keys) {
        $actual='accepted'
        try {$null=$Reader.Guard($name)} catch {$actual=$_.Exception.GetBaseException().Message}
        if($actual -cne $cases[$name]) {throw 'fixture-assertion'}
        $results.Add($name+':'+$actual)
    }
    return ,$results.ToArray()
}
