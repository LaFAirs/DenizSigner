#Requires -Version 5.1
<#
  Network audit: scans SHIPPED APP CODE (not docs/scripts) for outbound
  references and fails on telemetry SDKs, updater wiring, or upstream hosts.
  Docs (NOTICE/PRIVACY/README/ARCHITECTURE) may credit upstream in prose;
  they are not shipped network behavior and are excluded here.
  Usage: powershell -ExecutionPolicy Bypass -File scripts/audit-network.ps1
#>
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

function Get-CodeFiles {
  Get-ChildItem -Path (Join-Path $root "src"), (Join-Path $root "src-tauri") -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -notmatch '\\node_modules\\|\\target\\|\\dist\\' -and $_.Extension -match '\.(ts|tsx|rs|json|toml|html|css|svg)$' }
}

$rxUrl = 'https?://[A-Za-z0-9\.\-_/\+%~#?&=:@]+'
# Integration-grade tokens only: plain prose words like "analytics" in the
# Privacy UI text must NOT fail the audit.
$rxRed = 'sentry|posthog|plausible|umami|amplitude|mixpanel|hotjar|crashlytics|bugsnag|datadog|newrelic|appinsights|segment\(|telemetry\s*[:=]|analytics\s*[:=]|tracking\s*[:=]'

$fail = $false
$codeFiles = Get-CodeFiles

Write-Host "== URLs in shipped code =="
$hits = $codeFiles | Select-String -Pattern $rxUrl -AllMatches -ErrorAction SilentlyContinue
$seen = @{}
foreach ($h in $hits) {
  foreach ($m in $h.Matches) {
    $url = $m.Value.TrimEnd('"', "'", ')', ',', ';', '.')
    if ($url -match 'schema\.tauri\.app|w3\.org|localhost|127\.0\.0\.1') { continue }
    $key = ("{0}:{1} {2}" -f $h.Path.Replace($root, "."), $h.LineNumber, $url)
    if ($seen.ContainsKey($key)) { continue }
    $seen[$key] = $true
    Write-Host ("  {0}" -f $key)
    if ($url -match 'nab138|me\.nabdev|iloader\.app|gist\.githubusercontent\.com/nab138') {
      Write-Host "  !! upstream host in shipped code: $url" -ForegroundColor Red
      $fail = $true
    }
  }
}

Write-Host ""
Write-Host "== telemetry SDK tokens in shipped code =="
$red = $codeFiles | Select-String -Pattern $rxRed -ErrorAction SilentlyContinue
if ($red -and $red.Count -gt 0) {
  $red | ForEach-Object { Write-Host ("  {0}:{1}  {2}" -f $_.Path.Replace($root, "."), $_.LineNumber, $_.Line.Trim()) -ForegroundColor Red }
  $fail = $true
} else {
  Write-Host "  none found"
}

Write-Host ""
Write-Host "== updater wiring (must be absent) =="
$upd = @()
$upd += Select-String -Path (Join-Path $root "src-tauri/Cargo.toml"), (Join-Path $root "package.json") -Pattern 'updater' -ErrorAction SilentlyContinue
$upd += Select-String -Path (Join-Path $root "src-tauri/tauri.conf.json") -Pattern '"updater"|endpoints' -ErrorAction SilentlyContinue
$upd += $codeFiles | Select-String -Pattern 'plugin-updater|latest\.json|downloadAndInstall|checkForUpdates' -ErrorAction SilentlyContinue
if ($upd -and $upd.Count -gt 0) {
  $upd | ForEach-Object { Write-Host ("  {0}:{1}  {2}" -f $_.Path.Replace($root, "."), $_.LineNumber, $_.Line.Trim()) -ForegroundColor Red }
  $fail = $true
} else {
  Write-Host "  no updater references"
}

Write-Host ""
Write-Host "== allowlist module present =="
if (Test-Path (Join-Path $root "src-tauri/src/network_allowlist.rs")) { Write-Host "  network_allowlist.rs OK" }
else { Write-Host "  MISSING" -ForegroundColor Red; $fail = $true }

if ($fail) { Write-Host "`nAUDIT FAILED" -ForegroundColor Red; exit 1 }
Write-Host "`nAUDIT PASSED" -ForegroundColor Green
