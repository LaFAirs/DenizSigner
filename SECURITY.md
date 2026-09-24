# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

Open a **private security advisory** on GitHub
(`Security` → `Report a vulnerability`) or contact the maintainer directly.
Please include:

- affected version / commit
- steps to reproduce
- what you expected vs. what happened

Do **not** open public issues for vulnerabilities involving credentials,
signing material, or device pairing. You will get a first response within
72 hours.

## Never commit

- `.p12` / `.pfx` / `.mobileprovision` / `.cer` / private keys
- Apple IDs, passwords, 2FA codes, tokens, session cookies
- `.env` files with secrets

CI fails the build if such files are staged (`build.yml` → `No secrets committed`).

## Local-only by design

DenizSigner stores secrets exclusively in the OS credential store
(Windows DPAPI Credential Manager / macOS Keychain) under the service name
`denizsigner`, and redacts secrets from logs. See `PRIVACY.md`.
