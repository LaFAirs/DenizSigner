# DenizSigner on Linux

Native Tauri 2 / GTK / WebKitGTK desktop application. The existing React design,
Apple-ID authentication, provisioning, USB device handling, pairing management,
SideStore and LiveContainer installers remain available. No Windows runtime or VM.

## Install on Arch Linux

Runtime packages: `webkit2gtk-4.1`, `gtk3`, `openssl`, `libsecret`, `usbmuxd`,
and a Secret Service provider such as `gnome-keyring` or KeePassXC with its
Secret Service integration enabled. Unlock/create the encrypted keyring through
its system dialog. Never choose an empty keyring password. USB device connection
activates usbmuxd through the distribution's existing udev rules.

Extract the Linux tarball, then run `scripts/install-linux.sh` from the extracted
folder. This installs the executable under `~/.local/opt/denizsigner`, a launcher
under `~/.local/bin`, the original app icon and a desktop entry under
`~/.local/share/applications`. The installer adds the desktop ID to an existing
MacShell `~/.config/macdock/pinned.json`. No shell source changes are needed.
The application remains available after login/reboot. No reboot is needed for
installation. Use `GDK_BACKEND=wayland` in a Wayland session (the launcher does so).

Build from source with Rust stable, Node 22.12+ (24 recommended), npm and base-devel:
`npm ci && npm run build && cargo build --release --features custom-protocol --locked --manifest-path src-tauri/Cargo.toml`.
Package using `scripts/package-linux.sh`. The tarball contains an installer and
checksum; installed system libraries are required. Existing Windows release CI
is retained; Linux CI builds/tests separately and can publish a Linux prerelease.

## Credential Manager

Open **Credential Manager** in the management sidebar and unlock it with your OS
keyring. Import `.p12` / `.pfx`, give each profile a name and provide its password.
The UI displays subject, issuer, serial number, SHA-256 fingerprint, validity
interval and selected profile. Expired/not-yet-valid profiles can be inspected,
but cannot be selected or used for signing. Invalid archives, wrong passwords,
missing keys and certificate/key mismatches fail before any storage mutation.

The manager discovers candidate files in your home, Downloads, Documents,
Dokumente and the application's certificates folder (bounded, nonrecursive).
It never reads those files until you explicitly import one. Use the file picker
for all other locations. Removing a profile removes only its managed keyring
copy after confirmation; source certificates are never removed or revoked.

Passwords/tokens can be added and removed in the same UI; secret values never
appear in list responses. Apple login passwords and signing/pairing state also
use the OS credential store. The manager closes and clears visible fields on
lock or window close, and expires backend access after five minutes without a
credential operation. The OS may keep its own keyring unlocked for the login
session; the application's lock is an additional access gate, not an OS-wide
keyring lock. A new unlock retrieves the encrypted vault through Secret Service.

PKCS#12 bytes and passwords are stored together inside the OS-protected vault;
no plaintext filesystem fallback exists. Linux explicitly selects Secret Service
rather than a mock or volatile keyring backend. Decrypted secrets exist briefly
in process/library memory; owned secret strings/buffers are wiped on drop where
supported, while library and WebView copies cannot be guaranteed wiped. OS
keyring deletion removes live entries; SSD snapshots/backups cannot be securely
erased by an application. Do not commit private certificates or provisioning
profiles; these extensions are gitignored. Log events are redacted before both
the file and frontend sinks.

## Signing with imported profiles

Select a valid profile, then choose **IPA mit ausgewähltem Profil signieren …**.
Choose an input IPA, a matching `.mobileprovision` and a new output filename.
The application validates certificate/key matching, certificate and profile
expiry, authorized developer certificate and application identifier. It signs
Mach-O code and bundle resources using the existing open-source
`apple-codesign-quick` / RustCrypto implementation and embeds the supplied profile.
Secrets never enter command-line arguments or signing scratch files. Scratch
files are restricted to a private temporary directory; output is saved atomically
without overwriting an existing file.

This offline path exports the signed IPA. The existing Apple-ID path provisions,
signs and installs over USB separately. Imported-profile signing currently
rejects app extensions because each requires its own provisioning profile; use
the Apple-ID path for those IPAs. Apple devices require Apple-issued signing
certificates and valid Apple provisioning; a self-signed test certificate is
suitable only for verifying local cryptography, not for installing an iOS app.
The profile parser does not establish Apple trust/revocation; iOS remains the
final authority for installation. Avoid importing untrusted certificates/profiles.

## Validation

`npm test`, `npm run build`, `npm audit`, and
`cargo test --release --locked --manifest-path src-tauri/Cargo.toml`.
Rust tests generate their own test certificates and test bad passwords, expired
certificates, secret-free IPC summaries, lock expiry, IPA signing and independent
OpenSSL CMS signature verification. They never need personal signing keys.
The ignored `real_keyring_roundtrip` test additionally exercises the real OS
credential store; run explicitly only in a session with an unlocked keyring.
Physical-device and Apple-service workflows need a connected trusted iPhone and
an Apple account. They cannot be claimed verified by local crypto tests alone.
