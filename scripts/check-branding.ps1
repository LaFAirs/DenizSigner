#Requires -Version 5.1
<#
  Branding audit: fails on visible upstream references (iloader/nab138/nabdev).
  Allowed: docs that credit upstream (NOTICE.md, PRIVACY.md, README.md, ARCHITECTURE.md).
  Usage: powershell -ExecutionPolicy Bypass -File scripts/check-branding.ps1
#>
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

$fail = $false
$files = Get-ChildItem -Path $root -Recurse -File |
  Where-Object { $_.FullName -notmatch '\\node_modules\\|\\target\\|\\dist\\|\\.git\\|\\scripts\\' -and $_.Extension -match '\.(ts|tsx|json|rs|toml|html|css|svg|md|ps1|mjs|yml|yaml)$' }

foreach ($f in $files) {
  # Upstream credit lives only in these docs (license attribution).
  $creditDoc = $f.Name -match '^(NOTICE|PRIVACY|README|ARCHITECTURE)\.md$'
  # Locales + docs may use prose words ("analytics", "tracking") in privacy
  # statements; UI branding check targets product/owner names only here.
  if ($f.FullName -match '\\src\\locales\\') { continue }
  $text = [IO.File]::ReadAllText($f.FullName)
  foreach ($pat in @('iloader', 'iLoader', 'nab138', 'nabdev', 'me\.nabdev')) {
    $m = [regex]::Matches($text, $pat, 'IgnoreCase')
    foreach ($hit in $m) {
      # Allow mentions inside credit/audit docs and inside this script itself.
      if ($creditDoc -or $f.Name -eq 'check-branding.ps1' -or $f.Name -eq 'audit-network.ps1') {
        continue
      }
      Write-Host ("  {0}  matches '{1}'" -f $f.FullName.Replace($root, "."), $hit.Value) -ForegroundColor Red
      $fail = $true
      break
    }
  }
}

# Required new branding present?
$must = @(
  @{ file = "src-tauri/tauri.conf.json"; pattern = '"productName": "DenizSigner"' },
  @{ file = "src-tauri/tauri.conf.json"; pattern = '"identifier": "com.denizbudakli.denizsigner"' },
  @{ file = "index.html"; pattern = '<title>DenizSigner</title>' }
)
foreach ($m in $must) {
  $content = [IO.File]::ReadAllText((Join-Path $root $m.file))
  if ($content -notmatch [regex]::Escape($m.pattern)) {
    Write-Host ("  missing branding: {0} should contain {1}" -f $m.file, $m.pattern) -ForegroundColor Red
    $fail = $true
  }
}

if ($fail) { Write-Host "`nBRANDING CHECK FAILED" -ForegroundColor Red; exit 1 }
Write-Host "BRANDING CHECK PASSED" -ForegroundColor Green
