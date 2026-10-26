# Changelog

All notable changes to DenizSigner are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [0.1.2] — 2026-10-26

### Fixed

- Move to pinned `isideload` 0.5.0 and the current upstream Mac Anisette
  services. Fetch client identity from the selected service instead of the
  obsolete hardcoded Android workaround.
- Migrate legacy preset server preferences to `mac-ani.jkcoxson.com`;
  preserve custom server preferences. No credential deletion.
- Retry only an initial Apple login HTTP 503, twice, after 3 and 8 seconds.
  Never replay password proofs or 2FA requests. Disable upstream immediate
  HTTP 429 retries and show specific EN/DE Apple-service guidance.
- Preserve separate provisioning profiles for app extensions across the
  library API upgrade.

## [0.1.1] — 2026-10-26

### Fixed

- Apple login error `-80045`: pin `isideload` 0.4.5, including the upstream
  AuthKit identity fix, anisette resync and already-delivered 2FA handling.
- Register the code listener before opening the 2FA prompt, await it without
  blocking the async runtime, and remove it on timeout or event failure.
- Prevent overlapping login attempts; close expired 2FA prompts and preserve
  event subscriptions across React Strict Mode remounts.
- Show specific EN/DE guidance for remaining Apple `-80045` rejections.

### Known limitation

- The upstream compatibility fix may not support SMS 2FA. Use a verification
  code from a trusted Apple device. Apple account authentication and physical
  device installation still require an interactive user test.

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
