# Zapdude stable backup routine.
# Creates a timestamped backup folder under ..\backups\ with:
#   - the current debug build (zapdude.exe + zapdude.pdb)
#   - a source snapshot zip (whole project minus build outputs)
#   - a MANIFEST.txt recording what was verified so the snapshot deserves
#     the word "stable"
# Run this AFTER a verified build (tests green + user-facing checks done).
# Keeps the newest 12 folders.
param(
    [string]$Manifest = "no verification notes supplied"
)
$ErrorActionPreference = "Stop"
$root = "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs"
$backups = "C:\Users\PC\Desktop\Projects\Zapdude\backups"
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$dest = Join-Path $backups "stable-dev-$stamp"
New-Item -ItemType Directory -Path $dest -Force | Out-Null

# 1. The runnable build (the thing the user launches through run-dev.cmd).
Copy-Item "$root\target\debug\zapdude.exe" $dest -Force
if (Test-Path "$root\target\debug\zapdude.pdb") {
    Copy-Item "$root\target\debug\zapdude.pdb" $dest -Force
}

# 2. Source snapshot (build outputs excluded) via bsdtar (ships with Win10+).
Push-Location $root
try {
    & tar -a -c -f (Join-Path $dest "source.zip") --exclude "target" --exclude "*.log" --exclude "backups" . | Out-Null
} finally {
    Pop-Location
}

# 3. Manifest: what passed before this folder was taken.
$lines = @(
    "Zapdude stable backup",
    "taken: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')",
    "build: $(if (Test-Path (Join-Path $dest 'zapdude.exe')) { 'target\debug\zapdude.exe (dev build)' } else { 'DEVICE HAD NO EXE - INVALID SNAPSHOT' })",
    "manifest: $Manifest",
    ""
)
$lines | Out-File -FilePath (Join-Path $dest "MANIFEST.txt") -Encoding UTF8

# 4. Prune: keep the newest 12.
Get-ChildItem $backups -Directory | Sort-Object Name -Descending | Select-Object -Skip 12 | Remove-Item -Recurse -Force
Write-Output "backup ready: $dest"
