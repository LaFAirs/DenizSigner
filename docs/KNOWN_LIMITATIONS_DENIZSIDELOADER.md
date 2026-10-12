# Bekannte Einschränkungen (DenizSideloader-MVP)

1. **Keine Gerätetests bisher.** Signierung, Installation,
   Pairing-Ablage und Mitteilungen sind implementiert und Unit-getestet
   (Rust: `cargo test`, Swift: Simulator-Tests in CI), aber **nicht
   gegen ein echtes iPad verifiziert**. Bis dahin gilt jede
   Gerätefunktion als **ungetestet**.
2. **iPadOS 27 existiert öffentlich noch nicht** (Stand Okt 2026).
   Aussagen zu 27 sind Prognosen — Details in
   `docs/IPADOS27_COMPAT.md`.
3. **Kostenlose Apple-ID bleibt begrenzt**: 3 Apps, 10 App-IDs,
   ~7 Tage Gültigkeit. Die App verlängert nichts von selbst und
   schützt weder vor Widerruf noch vor Apples Limits.
4. **Keine Hintergrund-Autoerneuerung.** iOS garantiert keine
   Hintergrundausführung; die App verspricht Erinnerungen, keine
   automatische Verlängerung.
5. **Erneuerung braucht Windows + USB.** On-device-Signierung ist
   technisch nicht realisierbar und wird nicht behauptet.
6. **IPA-Parser best-effort**: kein Zip64, nur `Payload/*.app`-Bundles,
   200-MiB-Inflationscap. Unlesbare Archive werden behalten, aber als
   „unbekannt“ markiert statt zu raten.
7. **Windows-Build unsigniert** (SmartScreen-Warnung beim ersten Start),
   wie bisher bei DenizSigner.
8. Deutsche UI-Texte können bei neuen Schlüsseln zurückfallen (Englisch
   als Fallback), analog zum Desktop.
