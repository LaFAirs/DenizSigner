#Requires -Version 5.1
# Static verification for DenizSideloader MVP (no toolchain needed).
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$fail = $false

Write-Host "== JSON validity =="
foreach ($f in @("src/locales/en.json", "src/locales/de.json",
    "DenizSideloader/DenizSideloader/Localizable.xcstrings")) {
  try {
    Get-Content (Join-Path $root $f) -Raw | ConvertFrom-Json | Out-Null
    Write-Host ("  OK: {0}" -f $f)
  } catch {
    Write-Host ("  FAIL: {0} -> {1}" -f $f, $_) -ForegroundColor Red
    $fail = $true
  }
}

Write-Host ""
Write-Host "== locale key parity (denizsideloader keys) =="
$en = Get-Content (Join-Path $root "src/locales/en.json") -Raw | ConvertFrom-Json
$de = Get-Content (Join-Path $root "src/locales/de.json") -Raw | ConvertFrom-Json
foreach ($k in @("install_denizsideloader", "denizsideloader_hint")) {
  if ($en.app.$k -and $de.app.$k) { Write-Host ("  OK app.{0}" -f $k) }
  else { Write-Host ("  FAIL app.{0}" -f $k) -ForegroundColor Red; $fail = $true }
}
foreach ($k in @("install_denizsideloader_title", "install_denizsideloader_success_title",
    "install_denizsideloader_success_message", "install_denizsideloader_step_verify",
    "install_denizsideloader_step_install", "install_denizsideloader_step_pairing")) {
  if ($en.operations.$k -and $de.operations.$k) { Write-Host ("  OK operations.{0}" -f $k) }
  else { Write-Host ("  FAIL operations.{0}" -f $k) -ForegroundColor Red; $fail = $true }
}

Write-Host ""
Write-Host "== xcstrings key coverage vs Swift sources =="
$swift = (Get-ChildItem (Join-Path $root "DenizSideloader/DenizSideloader") -Recurse -Filter *.swift |
  Get-Content -Raw) -join "`n"
$xc = Get-Content (Join-Path $root "DenizSideloader/DenizSideloader/Localizable.xcstrings") -Raw | ConvertFrom-Json
$missing = @()
foreach ($m in [regex]::Matches($swift, '"([a-z][a-z0-9._-]*\.[a-z0-9._-]+)"')) {
  $key = $m.Groups[1].Value
  if ($key -match '^(home|apps|refresh|settings|badge|common|diag|ipa|pairing|transport)\.') {
    if (-not ($xc.strings.PSObject.Properties.Name -contains $key)) { $missing += $key }
  }
}
$missing = $missing | Sort-Object -Unique
if ($missing.Count -eq 0) { Write-Host "  OK: all namespaced keys present" }
else { $missing | ForEach-Object { Write-Host ("  MISSING: {0}" -f $_) -ForegroundColor Red }; $fail = $true }

Write-Host ""
Write-Host "== pbxproj reference integrity =="
$pbx = Get-Content (Join-Path $root "DenizSideloader/DenizSideloader.xcodeproj/project.pbxproj") -Raw
$refs = [regex]::Matches($pbx, 'fileRef = ([A-F0-9]{24});') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
$defs = [regex]::Matches($pbx, '^\s*([A-F0-9]{24}) = \{isa = PBX(FileReference|BuildFile)', 'Multiline') |
  ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
$dangling = $refs | Where-Object { $defs -notcontains $_ }
if ($dangling.Count -eq 0 -or $null -eq $dangling) { Write-Host "  OK: no dangling fileRefs" }
else { $dangling | ForEach-Object { Write-Host ("  DANGLING: {0}" -f $_) -ForegroundColor Red }; $fail = $true }
foreach ($f in @("DenizSideloaderApp.swift", "ContentView.swift", "Theme.swift",
    "Models/ManagedApp.swift", "Models/RefreshState.swift", "Services/ExpiryCalculator.swift",
    "Services/AppLibrary.swift", "Services/PairingManager.swift", "Services/Diagnostics.swift",
    "Services/KeychainStore.swift", "Services/DeviceTransport.swift", "Services/RefreshManager.swift",
    "Services/IpaParser.swift", "Views/HomeView.swift", "Views/AppsView.swift",
    "Views/RefreshView.swift", "Views/SettingsView.swift", "Localizable.xcstrings",
    "Assets.xcassets", "Info.plist", "DenizSideloader.entitlements")) {
  $base = Split-Path $f -Leaf
  if ($pbx.Contains(("path = {0};" -f $base))) { Write-Host ("  OK ref: {0}" -f $base) }
  else { Write-Host ("  MISSING ref: {0}" -f $base) -ForegroundColor Red; $fail = $true }
  if (Test-Path (Join-Path $root ("DenizSideloader/DenizSideloader/{0}" -f $f))) { }
  else { Write-Host ("  MISSING file: {0}" -f $f) -ForegroundColor Red; $fail = $true }
}

Write-Host ""
Write-Host "== Rust wiring =="
$lib = Get-Content (Join-Path $root "src-tauri/src/lib.rs") -Raw
foreach ($s in @("mod denizsideloader;", "install_denizsideloader_operation")) {
  if ($lib.Contains($s)) { Write-Host ("  OK lib.rs: {0}" -f $s) }
  else { Write-Host ("  FAIL lib.rs: {0}" -f $s) -ForegroundColor Red; $fail = $true }
}
$pair = Get-Content (Join-Path $root "src-tauri/src/pairing.rs") -Raw
if ($pair.Contains('("DenizSideloader", "ALTPairingFile.mobiledevicepairing")')) { Write-Host "  OK pairing.rs entry" }
else { Write-Host "  FAIL pairing.rs entry" -ForegroundColor Red; $fail = $true }
$ops = Get-Content (Join-Path $root "src/components/operations.ts") -Raw
if ($ops.Contains('id: "install_denizsideloader"')) { Write-Host "  OK operations.ts" }
else { Write-Host "  FAIL operations.ts" -ForegroundColor Red; $fail = $true }
$app = Get-Content (Join-Path $root "src/App.tsx") -Raw
if ($app.Contains("installDenizSideloaderOperation") -and -not $app.Contains("installSideStoreOperation")) {
  Write-Host "  OK App.tsx uses DenizSideloader only"
} else { Write-Host "  FAIL App.tsx wiring" -ForegroundColor Red; $fail = $true }

if ($fail) { Write-Host "`nSTATIC CHECKS FAILED" -ForegroundColor Red; exit 1 }
Write-Host "`nSTATIC CHECKS PASSED" -ForegroundColor Green
