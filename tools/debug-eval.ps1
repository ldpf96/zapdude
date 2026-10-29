# Zapdude debug eval helper.
# Usage: .\tools\debug-eval.ps1 -Script "<js expression>" [-Account account-1] [-All]
# Or:    .\tools\debug-eval.ps1 -ScriptFile some-script.js [-Account account-1]
param(
    [string]$Script,
    [string]$ScriptFile,
    [string]$Account,
    [switch]$All
)
if ($ScriptFile) {
    $path = $ScriptFile
    if ([System.IO.Path]::IsPathRooted($path) -eq $false) {
        $path = Join-Path $PSScriptRoot $ScriptFile
    }
    $Script = [System.IO.File]::ReadAllText($path)
}
if (-not $Script) { throw "script required (-Script or -ScriptFile)" }
$key = "zpk_846b54feb07ac04601129f85f3353cf73c6f9ac44c73c71e300740850251543d"
$payload = @{ script = $Script }
if ($Account) { $payload.account = $Account }
if ($All) { $payload.all = $true }
$body = $payload | ConvertTo-Json -Compress
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:29977/api/_debug/eval" `
        -Method Post -ContentType "application/json" `
        -Headers @{ "X-API-Key" = $key } -Body $body -UseBasicParsing -TimeoutSec 20
    Write-Output $response.Content
} catch {
    $stream = $_.Exception.Response.GetResponseStream()
    if ($stream) {
        $reader = New-Object System.IO.StreamReader($stream)
        Write-Output $reader.ReadToEnd()
    } else {
        Write-Output $_.Exception.Message
    }
}
