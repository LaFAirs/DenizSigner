# DenizSigner — Private iOS Sideloading & Signing Tool

Independent desktop app (Tauri 2 + React + Rust) for personal iOS sideloading.
Technical lineage: built from the open-source techniques of
[`nab138/iloader`](https://github.com/nab138/iloader) (MIT) — see `NOTICE.md`.
Own name, own `D` logo, own app ID (`com.denizbudakli.denizsigner`),
no updater, no telemetry, local-first.

## Quick start (Windows-first)

```powershell
cd DenizSigner
npm install
npm run tauri dev        # dev with hot reload (needs WebView2 + iTunes/Apple Devices + usbmuxd)
```

Production build (`.exe` / `.msi` via NSIS; `.app`/`.dmg` on macOS):

```powershell
# one-time: generate PNG/ICO/ICNS from the D logo
npx tauri icon src-tauri/icons/icon.svg
npm run tauri build
# output: src-tauri/target/release/bundle/...
```

The release binary shows no console window (`windows_subsystem = "windows"`).

## Checks

```powershell
npm run build            # tsc + vite
npm test                 # frontend checks (boot, IPA, errors, allowlist, branding)
powershell -ExecutionPolicy Bypass -File scripts/audit-network.ps1
powershell -ExecutionPolicy Bypass -File scripts/check-branding.ps1
cd src-tauri && cargo test
```

## Privacy

See [`PRIVACY.md`](PRIVACY.md) for the full network audit:
allowed = Apple (via `isideload`), your Anisette server, release IPAs you
explicitly request (github.com), Apple help links. Everything else is blocked
in `src-tauri/src/network_allowlist.rs`. No accounts, no analytics, no updater.

## Local data

| OS | Location |
|----|----------|
| Windows | `%APPDATA%\com.denizbudakli.denizsigner\` + `…\logs\` |
| macOS | `~/Library/Application Support/com.denizbudakli.denizsigner/` |
| Linux | `~/.local/share/com.denizbudakli.denizsigner/` |

Secrets (Apple password, Anisette state, pairing cache) live in the OS
credential store (DPAPI / Keychain) under service `denizsigner` — never in
plaintext JSON. `data.json` holds only Apple-ID e-mail addresses.
