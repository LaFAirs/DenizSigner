# NOTICE — Open-source components

DenizSigner contains code and techniques derived from the following
open-source projects. Their licenses are preserved; this file does not
replace them.

## nab138/iloader (MIT)

- Source: <https://github.com/nab138/iloader>
- License: MIT (see `LICENSE-MIT-UPSTREAM.txt` when vendored; upstream
  `LICENSE` applies to the code portions used as reference).
- Branding note: the upstream `LICENSE-BRANDING` reserves the name
  “iloader”, its logos and media. DenizSigner does **not** use them:
  new name, new geometric-D logo, new app identifier. No endorsement
  is implied.

## Dependency crates / npm packages

Licensed under their own MIT/Apache-2.0 terms (see lockfiles):

- `idevice` (jkcoxson) — iOS device communication
- `isideload` (nab138) — Apple auth / developer / signing flows
- Tauri 2 + official plugins (opener, store, dialog, process)
- React, Vite, i18next, sonner

## Own work

DenizSigner branding (name, D logo, blue UI), network boundary layer,
secret-redacting logger, IPA validator, friendly-error mapping, settings/
privacy pages and audit scripts are original work for personal use.
