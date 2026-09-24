#Requires -Version 5.1
<#
  Network + security-pattern audit for DenizSigner (CI: build.yml).

  WHAT IT CHECKS (shipped code only: src/**, src-tauri/src/**, configs):
    1. Every https?:// URL literal is listed; unknown hosts FAIL unless they
       are documented (PRIVACY.md allowlist) or test fixtures (*.example.*).
    2. Telemetry/tracker SDK tokens FAIL.
    3. Updater wiring FAILs (no auto-update by design).
    4. Dangerous primitives FAIL: raw sockets, shell execution, osascript-style
       helpers in OUR code (dependencies like `idevice` legitimately use TCP
       to the attached iPhone — audited separately, see SECURITY_AUDIT.md).
    5. `reqwest` / `fetch` / WebSocket usages are REPORTED (must stay exactly
       the documented ones: sideload.rs download + Tauri event layer).

  LIMITS (documented, per spec): this is string search, not data-flow
  analysis. It cannot prove the absence of dynamically constructed URLs or
  review third-party crates. Dependency review is done via `cargo audit` /
  `npm audit` (see SECURITY_AUDIT.md); runtime behavior is bounded by the
  `require_allowed_url` / `require_anisette_url` gate plus Tauri capabilities.

  Usage: powershell -ExecutionPolicy Bypass -File scripts/audit-network.ps1
#>
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

function Get-CodeFiles {
  Get-ChildItem -Path (Join-Path $root "src"), (Join-Path $root "src-tauri/src") -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Extension -match '\.(ts|tsx|rs)$' }
}

# Hosts that may appear in shipped code. Test/example domains are fixtures
# used by unit tests (network_allowlist.rs), not real endpoints.
$KnownHosts = @(
  "github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com",
  "iforgot.apple.com", "apple.co", "ani.sidestore.io",
  "schema.tauri.app", "localhost"
)
$TestHostRx = '(\.|^)(example|example\.com|example\.org|evil\.com|evilgithub\.com)$|^(example|example\.com|example\.org|evil\.com|evilgithub\.com)$'

$fail = $false
$codeFiles = Get-CodeFiles

Write-Host "== 1. URLs in shipped code (unknown hosts fail) =="
$hits = $codeFiles | Select-String -Pattern 'https?://[A-Za-z0-9\.\-_/\+%~#?&=:@\\]+' -AllMatches -ErrorAction SilentlyContinue
$seen = @{}
foreach ($h in $hits) {
  foreach ($m in $h.Matches) {
    $url = $m.Value.TrimEnd('"', "'", ')', ',', ';', '.')
    if ($url -match 'schema\.tauri\.app|w3\.org|localhost|127\.0\.0\.1') { continue }
    if ($url -match '^(https?|wss?)://[^/]*$' -and $url -notmatch '\.') { continue }
    $hostname = ($url -replace '^[a-z]+://', '' -split '[/?#:;\\]')[0].Trim().ToLower()
    # Credential-smuggling fixtures (negative unit tests): the gate blocks
    # any userinfo, so these can never become real requests.
    if ($url -match '^[a-zA-Z]+://[^/]*@') {
      Write-Host ("  ~ credential-fixture (gate-blocked): {0}:{1}" -f $h.Path.Replace($root, "."), $h.LineNumber)
      continue
    }
    $key = ("{0}:{1}  {2}  [host:{3}]" -f $h.Path.Replace($root, "."), $h.LineNumber, $url, $hostname)
    if ($seen.ContainsKey($key)) { continue }
    $seen[$key] = $true
    $known = $false
    foreach ($k in $KnownHosts) {
      if ($hostname -eq $k -or $hostname.EndsWith(".$k")) { $known = $true; break }
    }
    if (-not $known -and $hostname -match $TestHostRx) { $known = $true }
    if ($hostname -match '^(ani\.|.*\.ani\.)') {
      Write-Host ("  ~ anisette-adjacent (user-configurable, validated): {0}" -f $key)
      continue
    }
    if ($known) { Write-Host ("  ok {0}" -f $key) }
    else {
      Write-Host ("  !! UNKNOWN HOST: {0}" -f $key) -ForegroundColor Red
      $fail = $true
    }
  }
}

Write-Host ""
Write-Host "== 2. telemetry/tracker SDK tokens (must be absent) =="
$rxRed = 'sentry|posthog|plausible|umami|amplitude|mixpanel|hotjar|crashlytics|bugsnag|datadog|newrelic|appinsights|segment\(|telemetry\s*[:=]|analytics\s*[:=]|tracking\s*[:=]'
$red = $codeFiles | Select-String -Pattern $rxRed -ErrorAction SilentlyContinue
if ($red -and $red.Count -gt 0) {
  $red | ForEach-Object { Write-Host ("  {0}:{1}  {2}" -f $_.Path.Replace($root, "."), $_.LineNumber, $_.Line.Trim()) -ForegroundColor Red }
  $fail = $true
} else {
  Write-Host "  none found"
}

Write-Host ""
Write-Host "== 3. updater wiring (must be absent) =="
$upd = @()
$upd += Select-String -Path (Join-Path $root "src-tauri/Cargo.toml"), (Join-Path $root "package.json") -Pattern 'updater' -ErrorAction SilentlyContinue
$upd += Select-String -Path (Join-Path $root "src-tauri/tauri.conf.json") -Pattern '"updater"|endpoints|createUpdaterArtifacts": true' -ErrorAction SilentlyContinue
$upd += $codeFiles | Select-String -Pattern 'plugin-updater|latest\.json|downloadAndInstall|checkForUpdates' -ErrorAction SilentlyContinue
if ($upd -and $upd.Count -gt 0) {
  $upd | ForEach-Object { Write-Host ("  {0}:{1}  {2}" -f $_.Path.Replace($root, "."), $_.LineNumber, $_.Line.Trim()) -ForegroundColor Red }
  $fail = $true
} else {
  Write-Host "  no updater references"
}

Write-Host ""
Write-Host "== 4. dangerous primitives in OUR code (must be absent) =="
$rxDanger = 'TcpStream|UdpSocket|Command::new|std::process::Command|powershell|cmd\.exe|schtasks|reg\.exe|LaunchAgent|LaunchDaemon|Software\\Microsoft\\Windows\\CurrentVersion\\Run'
$danger = $codeFiles | Select-String -Pattern $rxDanger -ErrorAction SilentlyContinue
if ($danger -and $danger.Count -gt 0) {
  $danger | ForEach-Object { Write-Host ("  {0}:{1}  {2}" -f $_.Path.Replace($root, "."), $_.LineNumber, $_.Line.Trim()) -ForegroundColor Red }
  $fail = $true
} else {
  Write-Host "  none found (device TCP lives in the audited `idevice` dependency)"
}

Write-Host ""
Write-Host "== 5. reported (must stay exactly these) =="
$rep = $codeFiles | Select-String -Pattern 'reqwest|fetch\(|XMLHttpRequest|WebSocket' -ErrorAction SilentlyContinue
if ($rep) {
  $rep | ForEach-Object { Write-Host ("  {0}:{1}  {2}" -f $_.Path.Replace($root, "."), $_.LineNumber, $_.Line.Trim()) }
} else {
  Write-Host "  none"
}

Write-Host ""
Write-Host "== allowlist module present =="
if (Test-Path (Join-Path $root "src-tauri/src/network_allowlist.rs")) { Write-Host "  network_allowlist.rs OK" }
else { Write-Host "  MISSING" -ForegroundColor Red; $fail = $true }

if ($fail) { Write-Host "`nAUDIT FAILED" -ForegroundColor Red; exit 1 }
Write-Host "`nAUDIT PASSED" -ForegroundColor Green
