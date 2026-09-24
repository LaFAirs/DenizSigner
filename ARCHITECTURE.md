# ARCHITECTURE.md

```
Drop/IPA (.ipa) ──▶ validate_ipa_path ──▶ sideload_operation ──▶ isideload install ──▶ device (usbmuxd/lockdown)
Apple-ID login ──▶ anisette (user host, validated) ──▶ Apple (via isideload) ──▶ keyring (password, opt-in)
Pairing ──▶ lockdown + rppairing (17.4+) ──▶ keyring cache ──▶ place via house_arrest/AFC
Logs: tracing file (denizsigner*.log, 7 files) + redacted frontend mirror (log-record event, Info default)
```

## Backend (`src-tauri/src/`)

| File | Role | Tauri commands |
|------|------|----------------|
| `main.rs` | entry, no console on Win release, rustls + isideload init | – |
| `lib.rs` | builder: opener/store/dialog/process plugins (no updater), log setup, state | handler registry |
| `network_allowlist.rs` | **boundary**: `require_allowed_url`, `require_anisette_url`, `redact_url` + tests | – |
| `error.rs` | `AppError` + `friendly()` mapping + `anyhow` conversion + tests | – |
| `ipa.rs` | `.ipa` extension + ZIP `Payload/*.app` check + tests | – |
| `logging.rs` | `FrontendLoggingLayer` (level-gated), `redact()`, `set_log_level_debug` | `set_log_level_debug` |
| `secure_storage.rs` | keyring service `denizsigner`, `create_sideloading_storage` | `keyring_available`, `force_disable_keyring` |
| `account.rs` | Apple login (2FA via window events), certs, App IDs | `login_new`, `login_stored`, `delete_account`, `logged_in_as`, `invalidate_account`, `reset_anisette_state`, `get_certificates`, `revoke_certificate`, `list_app_ids`, `delete_app_id` |
| `device.rs` | usbmuxd device list/select (label `denizsigner`) | `list_devices`, `set_selected_device`, `cancel_pairing` |
| `pairing.rs` | lockdown/rppairing generate+cache, place/export/delete | `place_pairing_cmd`, `export_pairing_cmd`, `delete_stored_rppairing`, `has_stored_rppairing`, `installed_pairing_apps` |
| `sideload.rs` | validated sideload + gated SideStore/LiveContainer flow | `sideload_operation`, `install_sidestore_operation` |
| `operation.rs` | `operation_<id>` started/finished/failed events | – |

22 commands total — same surface as upstream, minus updater; plus
`set_log_level_debug`. Minimal permissions (`capabilities/default.json`).

## Frontend (`src/`)

- `App.tsx`: blue shell, header (D + DenizSigner + Settings/Logs/Device),
  device card, central drop card, 7-step progress, friendly errors + tech details.
- `components/`: `DeviceCard`, `SigningPanel` (2FA flow), `operations`
  (backend→UI step map), `LogContext` (sanitized mirror), `Modal`, `ConfirmDialog`.
- `pages/`: `SettingsDialog` (General/Signing/Privacy+allowlist/Advanced),
  `LogsDialog`, `AboutDialog`, `CertificatesDialog` (revoke confirm),
  `PairingDialog` (delete confirm).
- `lib/`: `api` (typed invoke wrappers), `errors` (friendly map + log
  sanitizer), `network` (`ALLOWED_SERVICES`, `isIpaPath`).
- `locales/`: `en`, `de` (structure kept extensible like upstream).

## Destructive-action confirmations

Revoke certificate, delete App ID (via list UI), delete stored pairing,
reset local configuration — all behind `ConfirmDialog` (Cancel / Confirm).
