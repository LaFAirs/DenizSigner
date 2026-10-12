# NOTICE — DenizSideloader

DenizSideloader (`DenizSideloader/`) is a clean-room implementation
written for this project. It contains **no code copied from SideStore,
LiveContainer, AltStore or any other sideloading app** — no renames,
no vendored files. Only Apple system frameworks are used
(SwiftUI, Foundation, Security, UserNotifications, Compression,
UniformTypeIdentifiers), which carry no redistribution obligations for
app source.

Ideas (not code) studied from public documentation: free-developer
7-day provisioning windows, pairing-file placement via AFC/HouseArrest
from the Windows side, local-expiry reminder UX.

The companion installer on the Windows side
(`src-tauri/src/denizsideloader.rs`) reuses DenizSigner's existing
MIT/Apache-2.0 dependencies (`idevice`, `isideload`) — see the
top-level `NOTICE.md`.
