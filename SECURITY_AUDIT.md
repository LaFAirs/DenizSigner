# SECURITY_AUDIT.md — DenizSigner security hardening (v0.1.0 → hardening pass)

Date: 2026-09-25. Scope: full repo (`src/`, `src-tauri/`, configs, scripts).
Goal: maximum transparency with **zero removed features**. No claim of
absolute security is made — see Residual risks.

Severity scale: Critical / High / Medium / Low / Informational.

## Findings and fixes

### 1. Anisette validation accepted paths/queries/fragments — Medium → fixed

Custom Anisette servers previously accepted any path/query/fragment and relied
only on an `https` scheme check. A pasted callback URL could change request
semantics. Now `require_anisette_url` enforces bare `https://host[:port]`:
no userinfo, path, query, fragment, backslash, whitespace, control/CRLF, or
non-ASCII (punycode `xn--` allowed); ports 1–65535 pass through for
self-hosted servers. Covered by 20+ unit assertions.

### 2. Downloads used one-shot `reqwest::get` — Medium → fixed

Redirects were followed blindly and the whole body buffered in RAM.
`download()` now follows redirects manually (max 5 hops, every hop
re-validated against the allowlist, so GitHub → CDN works and arbitrary
hosts are blocked), streams the body with a 512 MiB cap, truncates (never
appends to) the fixed temp filename, and never logs query strings.

### 3. IPA pre-check missed traversal/symlink/size gates — Medium → fixed

`validate_ipa_path` now additionally rejects `..`, absolute and
drive-letter entries, backslash escapes, symlink entries (unix type bits),
>100k entries, and >8 GiB total uncompressed size — before `isideload`
ever touches the file. Unit-tested with hostile archives.

### 4. Password/2FA lifetime in memory — Low → hardened

Passwords are wrapped in `zeroize::Zeroizing` on arrival in
`login_new`/`login_stored` and wiped after the login call; the 2FA code is
wrapped the same way in the callback. Frontend clears the password field
after every attempt and never persists codes. Residual copies inside
`isideload`/keyring/OS internals are outside our control (documented below).

### 5. Account deletion left the live session behind — Low → fixed

`delete_account` removed keyring + metadata but kept a live in-memory
session for the deleted address. It now invalidates the session only when
it belongs to the deleted address (`should_invalidate`, case-insensitive,
unit-tested). The shared anisette state is intentionally kept — it is not
per-account, wiping it would affect other accounts.

### 6. Revoke/delete without confirmation — Low → fixed

Certificate revoke and App-ID delete now go through the existing confirm
dialog (Cancel/Confirm). Pairing export already did.

### 7. `rsa 0.9.10` (Marvin timing sidechannel, RUSTSEC-2023-0071) — Low, accepted

Pinned via `apple-codesign-quick → isideload` (core signing path — replacing
it would risk breaking Apple signing). The attack needs a timing oracle over
many decryptions; DenizSigner performs RSA locally on user-owned keys with
no network-exposed oracle. No patched 0.9.x exists that the dependency tree
accepts. Revisit on `isideload` updates.

### 8. Transitive maintenance warnings — Informational, accepted

`cargo audit`: `proc-macro-error`, `unic-*` (unmaintained), `glib`
unsoundness (Linux-only path, not built on Windows). No exploit path in this
app; tracked, revisited on dependency updates.

### 9. npm dev-only advisories — Informational, accepted

`npm audit --omit=dev`: 0 vulnerabilities. Full `npm audit`: esbuild/vite
dev-server issues (GHSA-67mh-4wv8-2f99 and vite ≤6.4.2 path issues) — dev
server binds localhost only, never shipped (`dist/` is static). No
breaking-change-free fix on vite 5.4.x; documented, not ignored.

## No-issue verifications (with evidence)

- No `println!`/`dbg!`/`eprintln!` in `src-tauri/src` (grep, 0 hits).
- No `Command::new`/`std::process`/shell tools/schtasks/reg/Run-keys/
  LaunchAgents in our code (grep, 0 hits). Browser opening uses the audited
  `tauri-plugin-opener` only.
- No raw `TcpStream`/`UdpSocket` in our code (grep, 0 hits). Device TCP
  (usbmuxd localhost:27015, lockdown/AFC tunnels, RSD handshake) lives in
  the `idevice` dependency and only runs after the user selects a device.
- No autostart/persistence installed (no Run keys, services, tasks,
  LaunchAgents/Daemons in code or bundle config).
- No `.env` files, no hardcoded credentials/keys/tokens (grep + CI gate).
- `data.json` holds only Apple-ID e-mails; the frontend never writes
  passwords anywhere (asserted in `scripts/test-frontend.mjs`).
- Tauri capabilities: `core/opener/store/dialog/process(allow-restart)`
  only — no fs/shell/http/window/event extras; updater plugin absent.

## Allowed network hosts

Enforced in `src-tauri/src/network_allowlist.rs`, documented row-by-row in
`PRIVACY.md` (machine-checked by `privacy_documents_all_hosts` test):

- Apple internals via `isideload` (auth/developer/signing) — on user action
- Configured Anisette host, exact match (default `ani.sidestore.io`)
- `github.com`, `objects.githubusercontent.com`,
  `release-assets.githubusercontent.com` — explicit IPA installs + redirects
- `iforgot.apple.com`, `apple.co` — browser help links

Everything else is blocked; `http:`, userinfo, backslash, control chars,
non-ASCII, fragments, and (for downloads) explicit ports are rejected.

## External dependencies (security-relevant)

| Dependency | Role | Note |
| ---------- | ---- | ---- |
| `isideload 0.4.0` | Apple auth/developer/signing | audited via `cargo audit`; pins `rsa` (see §7) |
| `idevice 0.1.68` | USB/lockdown/AFC/pairing | device TCP only, user-triggered |
| `reqwest 0.12` (+`stream`) | IPA downloads | gated URLs, manual redirects, size cap |
| `keyring 3.6` (native) | DPAPI/Keychain secrets | no plaintext fallback by default |
| `zip 2` | IPA pre-validation | read-only scan, traversal/symlink/size gates |
| `zeroize 1` | in-memory secret wiping | passwords + 2FA codes |
| `url 2` | strict URL parsing for the gate | — |
| Tauri 2 + opener/store/dialog/process plugins | app shell | minimal capabilities, no updater |
| React/Vite/i18next/sonner (+icons/virtuoso) | UI | 0 prod vulnerabilities |

## Where Apple credentials are processed

1. UI input (`AppleID.tsx` state, cleared after attempt) → `login_new`
   command arg → `Zeroizing` → `isideload` login (+ optional keyring save).
2. Keyring (`service=denizsigner`, account=e-mail) → `login_stored` →
   `Zeroizing` → `isideload` login.
3. 2FA: backend `2fa-required` event → UI modal → `2fa-recieved` event →
   `Zeroizing` → `isideload` response. Never logged, never stored.
4. `delete_account`: removes keyring entry + `data.json` id + matching live
   session. Anisette state kept (shared, documented).
5. Passwords never enter logs (redact gate), error messages (only error
   *types*/messages from Apple, no secrets), `data.json`, or crash dumps
   beyond OS-level process memory.

## Test results (this pass)

- `cargo test`: 19+ unit tests — allowlist matrix (hosts, schemes,
  userinfo, backslash, CRLF, unicode, ports, fragments, lookalikes),
  anisette matrix, redirect/size helpers, IPA traversal suite, redaction
  suite (password/2FA/token/query), account-invalidation suite.
- `npm test`: frontend checks incl. password-clearing, no secret storage,
  revoke/delete confirms, security section, anisette warning, locale keys.
- `scripts/audit-network.ps1`: extended (unknown-host detection,
  dangerous primitives, reported-reqwest inventory, documented limits).
- Manual device flows (login/2FA/signing/install on real iPhone):
  NOT TESTED — no device attached (honest limitation, see main report).

## Known limitations

- Residual secret copies inside `isideload`/keyring/OS memory cannot be
  wiped by us; lifetime is minimized, never persisted outside the keyring.
- The audit script is string search, not flow analysis; dependency internals
  are covered by `cargo audit`/`npm audit`, not line review.
- 2FA modal has no cancel path by design (upstream flow); backend enforces a
  120 s timeout, the code never persists.
- `glib`/`unic-*`/`proc-macro-error` warnings and the vite/esbuild dev-only
  advisories are accepted risks (above), re-checked on updates.
