# DenizSideloader

Minimal iOS/iPadOS companion to **DenizSigner** (Windows). Tracks managed
IPAs, provisioning expiry and pairing files **on device**. Signing and
(re-)installation run on Windows via DenizSigner over USB.

- SwiftUI, dark-blue minimal design, iPhone + iPad, German + English
- No accounts, no analytics, no ads, no cloud
- MIT, clean-room implementation (no third-party app code)

> Honest scope: a sandboxed iOS app cannot install IPAs or re-sign other
> apps by itself. This app never pretends otherwise: install/refresh
> actions route to the Windows companion with exact instructions, while
> expiry tracking, reminders, pairing checks and diagnostics run locally.
> Free Apple IDs stay bound to Apple's limits (3 apps, 10 App IDs,
> ~7-day expiry). See `docs/IPADOS27_COMPAT.md`.

## Build (macOS + Xcode)

Requirements: macOS with Xcode 16+, iOS 17 SDK.

```sh
cd DenizSideloader
xcodebuild test \
  -project DenizSideloader.xcodeproj \
  -scheme DenizSideloader \
  -destination 'platform=iOS Simulator,name=iPhone 16,OS=latest' \
  CODE_SIGNING_ALLOWED=NO

xcodebuild \
  -project DenizSideloader.xcodeproj \
  -scheme DenizSideloader \
  -configuration Release \
  -sdk iphoneos \
  -derivedDataPath build/derived \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO \
  build
./package-ipa.sh
# -> DenizSideloader-unsigned.ipa
```

No macOS at hand? The `iOS` GitHub Actions workflow (`macos-15`)
runs the same steps and uploads `DenizSideloader-unsigned` as artifact.

## Install on the iPad (Windows)

Full 8-step flow: `docs/DENIZSIDELOADER_INSTALL.md`.

1. iPad per USB verbinden, entsperren, **Vertrauen** tippen.
2. In DenizSigner Gerät auswählen (Pairing läuft automatisch).
3. `DenizSideloader-unsigned.ipa` aus dem CI-Artefakt bereitlegen.
4. **Install DenizSideloader** klicken, IPA wählen, mit Apple-ID (2FA) anmelden.
5. DenizSigner signiert (`isideload`) und installiert (`idevice`).
6. Die Pairing-Datei wird automatisch in DenizSideloader abgelegt.
7. DenizSideloader öffnen: Pairing vorhanden, Ablaufdatum ~+7 Tage.
8. Fertig — Refresh-Tab zeigt die nächste Fälligkeit.

## Refresh (7-Tage-Fenster)

- Der Refresh-Tab listet fällige Apps (≤ 2 Tage Rest), plant lokale
  Erinnerungen und zeigt die letzte erfolgreiche Verlängerung.
- Das Erneuern selbst läuft per USB in DenizSigner; danach in der App
  **Als erneuert markieren** tippen.
- iOS garantiert keine Hintergrund-Ausführung — die App verspricht
  deshalb Erinnerungen, keine automatische Erneuerung.

## Project layout

```
DenizSideloader/
├── DenizSideloader.xcodeproj/   # hand-written, dependency-free project
├── DenizSideloader/             # SwiftUI app (App, Theme, Models, Services, Views)
├── DenizSideloaderTests/        # expiry + pairing/diagnostic failure tests
├── package-ipa.sh               # unsigned-IPA packaging for DenizSigner
├── README.md · NOTICE.md
```

Bundle ID: `com.denizbudakli.denizsideloader` (must match
`DENIZSIDELOADER_BUNDLE_ID` in `src-tauri/src/denizsideloader.rs`).

## License

MIT — see `NOTICE.md` (no third-party app code included).
