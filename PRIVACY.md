# PRIVACY.md — Network & storage audit (DenizSigner v0.1.0)

## Principle

Local-first. No accounts, no analytics, no telemetry, no crash reporting to
third parties, no remote feature flags, no auto-updater, no CDN runtime
dependencies. Every outbound connection is allowlisted in code
(`src-tauri/src/network_allowlist.rs`) and enforced for DenizSigner's own
requests.

## Upstream reference audit (nab138/iloader @ 2.3.4)

Method: full-text search over `src/`, `src-tauri/`, configs for
`fetch|axios|reqwest|http|telemetry|analytics|sentry|tracking|webhook|
statistics|update-checker|posthog|plausible|umami`.

Result: **no telemetry/tracking/crash-reporting exists upstream.**
Found external touchpoints:

| # | Endpoint | Code location | Purpose | DenizSigner |
|---|----------|---------------|---------|-------------|
| 1 | `github.com/nab138/iloader/releases/.../latest.json` (+ `pubkey`) | `tauri.conf.json:30`, `src/update.ts` (tauri-plugin-updater) | auto-update manifest | **REMOVED** — updater plugin + `update.ts` deleted; `createUpdaterArtifacts: false`; audit script fails on `latest.json` |
| 2 | `api.github.com/repos/nab138/iloader/...` + gist badge | `count_downloads.ts`, README badge | CI download counter | **REMOVED** — script not shipped |
| 3 | `github.com/SideStore/SideStore`, `github.com/LiveContainer/...` `.ipa` | `sideload.rs:109-125` via `reqwest::get` | user-clicked IPA downloads | **KEPT, gated** — only on explicit click, URL checked by `require_allowed_url` (github.com + asset hosts, https only) |
| 4 | Anisette: `ani.sidestore.io` (default) + alternatives, user-custom | `Settings.tsx:25-34`, `account.rs:192-204` | Apple-auth helper headers | **KEPT, validated** — `require_anisette_url` (https + host); user-configurable; documented in Settings → Privacy |
| 5 | Apple private endpoints (GSA, developer portal) via `isideload` | `account.rs:205-210` (`AppleAccount::login`, `DeveloperSession`) | login, certs, App IDs, signing | **KEPT** — functionally required; no DenizSigner-owned server in between |
| 6 | `iforgot.apple.com`, `apple.co/ms`, sidestore release/help links | locales, `openUrl` calls | help links (browser) | **KEPT** — browser navigation only, no background traffic |
| 7 | `github.com/nab138/iloader`, `discord.gg/…`, `iloader.app` links | About/errors/docs | upstream help/community | **REMOVED/REPLACED** — About shows local info + NOTICE.md; no upstream links in UI |

Device-local traffic (never internet): usbmuxd/lockdown/AFC/house-arrest
over USB (`device.rs`, `pairing.rs`) — polled on start, user refresh, or
device change only; no background polling loops.

## Remaining necessary connections

1. **Apple** (through `isideload`, no DenizSigner server involved):
   Apple-ID login + 2FA session, developer session, certificates, App IDs,
   provisioning, signing/install handshake. Data: credentials in transit
   only; password never written to disk outside the OS keyring (and only
   with explicit opt-in).
2. **Anisette server** (default `https://ani.sidestore.io`, changeable):
   protocol helper for Apple auth. Data: routing identifiers the Apple
   protocol requires. No DenizSigner telemetry added.
3. **github.com release assets**: only the exact `.ipa` the user clicks
   (SideStore / LiveContainer). Data: requested bytes. Local `.ipa` import
   (Choose IPA / drag & drop) needs **no** network at all.
4. **Help links**: opened in the system browser on click.

## Documented hosts (machine-checked)

This table is the contract: `documented_hosts()` in
`src-tauri/src/network_allowlist.rs` lists these exact strings, and the
`privacy_documents_all_hosts` unit test fails the build if any of them stops
appearing in this file. Add a host to the code only together with a row here.

| Host | Zweck | Wann | Daten |
| ---- | ----- | ---- | ----- |
| Apple (hosts internal to `isideload`: GSA auth, developer portal) | Login / Developer API (Apple-ID, 2FA-Session, Zertifikate, App-IDs, Provisioning, Signing) | bei Apple-Funktionen (Login, Zertifikate, Install) | notwendige Auth-/Developer-Daten, nur im Transit |
| ani.sidestore.io | Anisette (Default-Server, in Settings änderbar) | beim Login/Signing | Anisette-bezogene Routing-Daten des Apple-Protokolls |
| github.com | IPA-Download (SideStore-/LiveContainer-Releases) | nur nach Benutzeraktion (Install-Klick) | Download-Anfrage (Dateiname), Antwort-Bytes |
| objects.githubusercontent.com | GitHub-CDN als Redirect-Ziel von github.com | nur nach Benutzeraktion (Redirect) | Download-Anfrage, Antwort-Bytes |
| release-assets.githubusercontent.com | GitHub-CDN als Redirect-Ziel von github.com | nur nach Benutzeraktion (Redirect) | Download-Anfrage (signierte URL ohne Logging), Antwort-Bytes |
| iforgot.apple.com | Hilfe-Link (Account-Entsperrung), Browser | nur nach Klick | keine (Browser-Navigation) |
| apple.co | Hilfe-Link (iTunes-Download), Browser | nur nach Klick | keine (Browser-Navigation) |
| andere | — | — | keine (blockiert, siehe `require_allowed_url`) |

## Storage

| What | Where |
|------|-------|
| App data + `logs/denizsigner*.log` (7 daily files) | `%APPDATA%\com.denizbudakli.denizsigner\` (Win), `~/Library/Application Support/com.denizbudakli.denizsigner/` (macOS), `~/.local/share/com.denizbudakli.denizsigner/` (Linux) |
| Apple password, Anisette state, pairing cache | OS credential store, service `denizsigner` (DPAPI / Keychain). Fallback to disk only with explicit user toggle + warning |
| Apple-ID e-mails (not secrets), prefs (`anisetteServer`, `lang`, log level) | `data.json` / `preferences.json` under app-data; UI prefs additionally in `localStorage` |

## Logging

Levels: Error / Warning / Info / Debug. Default **Info**; Debug via
Settings → Advanced (`set_log_level_debug`). `redact()` strips
password/token/2FA/session/cookie/private-key fragments before any sink.
2FA codes and passwords are never logged.

## Verification

```powershell
powershell -ExecutionPolicy Bypass -File scripts/audit-network.ps1
powershell -ExecutionPolicy Bypass -File scripts/check-branding.ps1
cd src-tauri && cargo test   # allowlist, IPA, error-mapping, redaction, pairing-version tests
npm test                     # frontend checks
```
