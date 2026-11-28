param([string]$Nonce)
# TDD red fixture: live entry is denied; owned fixture assertions expose missing sealing.
function New-ReadBudget([long]$Maximum) { return @{maximum=$Maximum;hashed=0} }
function Read-SealedFile($File,$Budget) { return @{sha256=''} }
if ($MyInvocation.InvocationName -ne '.') { exit 1 }
