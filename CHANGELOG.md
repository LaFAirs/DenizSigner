# Changelog

All notable changes to DenizSigner are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- DenizSideloader MVP: own minimal SwiftUI companion app
  (`DenizSideloader/`, MIT, clean-room) with Home / Apps / Refresh /
  Settings, expiry tracking, local reminders, pairing checks and
  diagnostics (DE + EN, iPhone + iPad)
- DenizSigner installs DenizSideloader via new **Install
  DenizSideloader** flow (local unsigned IPA + proven USB transport +
  automatic pairing placement); SideStore UI removed, backend kept as
  documented recovery path (`docs/RECOVERY_SIDESTORE.md`)
- iOS CI workflow (macOS runner): Swift unit tests + unsigned-IPA
  artifact for Windows signing
- Docs: install workflow, iPadOS-27 compatibility matrix, known
  limitations; all device functions marked untested until real-device
  verification

### Fixed

- Header layout hardened (wrapping nav, no clipped actions on small windows)

## [0.1.0] — 2026-09-24

### Added

- Initial DenizSigner release (Windows x64: portable `.exe`, NSIS `-setup.exe`, `.msi`)
- iloader-style workspace UI with DenizSigner branding (D logo, blue theme, EN/DE)
- IPA import via file picker and drag & drop, with local validation
  (`.ipa` extension, ZIP structure, `Payload/*.app`)
- SideStore / LiveContainer one-click installers with pairing-file placement
- Apple-ID sign-in with 2FA flow, saved logins, max-certificate picker
- Development certificate management (list, inspect, revoke with confirmation)
- App ID management (list, quota display, delete with confirmation)
- Pairing management (place, place-in-all, export with confirmation)
- Settings: Anisette presets + custom server, language, keyring toggle,
  anisette reset, log viewer with level filter, privacy/network section
- About dialog with version and open-source credits
- Local-first privacy model: OS credential store (DPAPI/Keychain), secret
  redaction in logs, enforced network allowlist, no telemetry, no updater
- Audit scripts (`audit-network.ps1`, `check-branding.ps1`) and test suites
  (53 frontend checks, 11 Rust unit tests)
- GitHub Actions: build/test/audit CI and tag-triggered releases with
  `SHA256SUMS.txt`
