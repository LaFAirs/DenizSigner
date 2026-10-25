# Linux validation — 2026-10-25

Host: Arch Linux x86_64, Hyprland, MacShell. Checks were actually executed
against the installed release binary; generated test keys only.

| Check | Result |
| --- | --- |
| TypeScript + Vite production build | PASS |
| Existing frontend checks | 72 / 72 PASS |
| Rust release tests | 23 PASS; 1 real-keyring test separately enabled |
| Real Rust keyring write/read/delete | PASS |
| Independent libsecret write/read/delete | PASS |
| Native Hyprland launch | PASS, `xwayland: false` |
| Embedded UI with no development server | PASS; `custom-protocol` build |
| GUI unlock against GNOME Keyring | PASS |
| GUI PKCS#12 import, metadata and persistent selection | PASS |
| GUI wrong certificate password | PASS; existing profile unchanged |
| GUI offline IPA signing with selected profile | PASS |
| Independent OpenSSL verification of GUI-exported CMS signature | PASS |
| GUI profile deletion with confirmation | PASS; source PKCS#12 retained |
| GUI token storage and confirmed deletion | PASS |
| Test passwords/token marker absent from app logs | PASS |
| npm audit | 0 reported vulnerabilities |
| App data / keyring permissions | 0700 |
| Installed executable matches release build SHA-256 | PASS |
| No-device startup with inactive usbmuxd | PASS; empty device list, no error modal |

GUI workflow tests ran in an isolated Xvfb display after a real Wayland launch
was verified, so synthetic keyboard input did not interfere with the user's
other applications. The secret store was the actual user's encrypted GNOME
Keyring throughout, never a mock backend. Synthetic RSA signing identities,
matching test provisioning profiles and a minimal ARM64 Mach-O IPA were generated
locally. Test-only profiles/tokens were removed through confirmation dialogs.
No original certificates were deleted, read, uploaded or used for signing.

The installable archive is built with the original application icon replaced by
the user's requested rounded light-blue icon. Windows/macOS icon formats were
regenerated with the official Tauri icon command. The Linux CI workflow builds,
tests and packages independently of the existing Windows release workflow.

Limits: no physical iPhone is currently attached, so live Apple login, Apple
provisioning, installation/trust and pairing on a device were not tested. A
self-signed test identity proves local cryptography, not acceptance by iOS.
A reboot was not performed because the desktop is in active use. Persistence
is implemented through permanent files and the existing Secret Service systemd
socket / DBus activation. Windows/macOS builds were not executed on this Linux
host. Imported-profile export currently rejects app extensions requiring their
own provisioning profiles; the original Apple-ID workflow remains available.
