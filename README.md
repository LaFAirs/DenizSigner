# DenizSigner

**Private iOS Sideloading & Signing Tool** — a local-first desktop app for
personally sideloading IPAs onto iPhone & iPad. No accounts, no analytics,
no telemetry, no auto-updater.

[![Build](https://github.com/LaFAirs/DenizSigner/actions/workflows/build.yml/badge.svg)](https://github.com/LaFAirs/DenizSigner/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/LaFAirs/DenizSigner?display_name=tag)](https://github.com/LaFAirs/DenizSigner/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/LaFAirs/DenizSigner/total)](https://github.com/LaFAirs/DenizSigner/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Windows x64](https://img.shields.io/badge/Windows-x64-blue.svg)](#download)

## Download

Get the latest release for Windows x64 from
[**GitHub Releases**](https://github.com/LaFAirs/DenizSigner/releases/latest):

| File | What |
| ---- | ---- |
| `DenizSigner_*_x64-setup.exe` | Installer (NSIS, recommended) |
| `DenizSigner_*_x64_en-US.msi` | Installer (MSI) |
| `denizsigner.exe` | Portable, no install |
| `SHA256SUMS.txt` | Checksums for everything above |

Verify integrity before installing:

```powershell
certutil -hashfile DenizSigner_0.1.0_x64-setup.exe SHA256
# compare with the matching line in SHA256SUMS.txt
```

You need the free **Apple Devices** app (or iTunes) from the Microsoft Store
so Windows can talk to your iPhone/iPad over USB.

## Features

- 📲 **IPA import & install** — file picker (`.ipa` filter) plus SideStore /
  LiveContainer one-click installers with automatic pairing-file placement
- 🔑 **Apple-ID sign-in** — 2FA flow, saved logins, max-certificate picker
- 📜 **Certificates & App IDs** — list, inspect, revoke (with confirmation)
- 🔗 **Pairing management** — place / place-in-all / export (confirmed)
- 🧰 **Settings** — Anisette server presets + custom, language (EN/DE),
  keyring toggle, anisette reset, log viewer with level filter
- 🔒 **Privacy section** — in-app overview of every allowed network endpoint
- 🪟 **No console window**, native `.exe` / `.msi`, German + English UI

## Privacy

Local-first. Secrets live only in the OS credential store
(Windows DPAPI / macOS Keychain, service `denizsigner`); logs redact secrets;
devices are polled on start / refresh only. The full network audit —
every allowed domain, why it is needed, and what was removed — is in
[`PRIVACY.md`](PRIVACY.md). The boundary is enforced in code
(`src-tauri/src/network_allowlist.rs`) and checked by CI.

## Development (Windows-first)

```powershell
npm install
npm run tauri dev
```

Requirements: Node 24+, Rust stable, VS Build Tools 2022 (VCTools),
Windows SDK 10, WebView2 Runtime.

```powershell
npx tsc --noEmit   # typecheck
npm run build      # frontend bundle
npm test           # 53 frontend checks
cd src-tauri && cargo test   # 11 backend tests
```

Releases are cut from version tags (`git tag v0.1.0 && git push origin v0.1.0`);
see [`CONTRIBUTING.md`](CONTRIBUTING.md). Icons: `npx tauri icon src-tauri/icons/icon.svg`.

## Credits

Built on open-source components — see [`NOTICE.md`](NOTICE.md) —
including the techniques of
[`nab138/iloader`](https://github.com/nab138/iloader) (MIT),
[`idevice`](https://github.com/jkcoxson/idevice),
[`isideload`](https://github.com/nab138/isideload), Tauri, and React.
DenizSigner uses its own name, logo, app ID (`com.denizbudakli.denizsigner`),
and UI theme; no endorsement is implied.

## License

MIT — see [LICENSE](LICENSE). Security policy: [SECURITY.md](SECURITY.md).
