# Changelog

All notable changes to DenizSigner are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
