# Contributing to DenizSigner

## Workflow (Windows-first)

```powershell
git clone https://github.com/LaFAirs/DenizSigner.git
cd DenizSigner
npm install
npm run tauri dev
```

You need: Node 24+, Rust stable, VS Build Tools 2022 (VCTools workload),
Windows SDK 10, WebView2 Runtime, and for devices Apple's `Apple Devices`
app / iTunes (usbmuxd).

## Before every PR

```powershell
npx tsc --noEmit
npm run build
npm test
powershell -ExecutionPolicy Bypass -File scripts/audit-network.ps1
powershell -ExecutionPolicy Bypass -File scripts/check-branding.ps1
cd src-tauri && cargo test
```

## Rules

- **No new network endpoints** without documenting them in `PRIVACY.md` and
  enforcing them in `src-tauri/src/network_allowlist.rs`.
- **No telemetry, analytics, crash-reporting, or update-checkers.** Ever.
- **No upstream branding** (former product or owner names) in shipped code or UI.
  Upstream credit lives only in `NOTICE.md` / `PRIVACY.md` / `ARCHITECTURE.md`.
- **No secrets in code, logs, or commits.** Passwords/tokens/keys belong in
  the OS credential store; logs must pass through `redact()`.
- **Destructive actions need confirmation dialogs** (revoke, delete, reset).
- Keep the UI layout faithful to the reference design; branding, strings
  (EN/DE), and docs stay DenizSigner.

## Releases

Maintainers cut releases from version tags:

```powershell
git tag v0.2.0; git push origin v0.2.0
```

`release.yml` builds, signs nothing (desktop app), attaches
`.exe` + `-setup.exe` + `.msi` + `SHA256SUMS.txt` to the GitHub Release.
