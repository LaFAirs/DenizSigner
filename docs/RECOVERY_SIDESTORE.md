# Recovery: SideStore (Legacy)

Die SideStore-/LiveContainer-Installation ist seit DenizSideloader aus
der UI entfernt, aber **als Rückweg funktionsfähig** gehalten:

- Backend: `install_sidestore_operation` in `src-tauri/src/sideload.rs`
  ist unverändert vorhanden (Download + Signierung + Pairing-Ablage).
- Pairing-Pfade für SideStore/LiveContainer bleiben in
  `src-tauri/src/pairing.rs` (`PAIRING_APPS`) registriert.
- Operation-Definitionen (`installSideStoreOperation`,
  `installLiveContainerOperation`) bleiben in
  `src/components/operations.ts` exportiert.

## Wiederherstellung (falls DenizSideloader fehlschlägt)

1. In `src/App.tsx` die Legacy-Operationen wieder einhängen (Buttons
   wie vor dem DenizSideloader-Commit) — keine Backend-Änderung nötig.
2. Alternativ: SideStore-IPA manuell von GitHub laden und über
   **Import IPA** installieren, danach Pairing über
   **Manage Pairing File → Place** ablegen.
3. Fehler in beiden Wegen: Logs sichern (Settings → View Logs),
   Issue ohne Passwörter/Codes/Tokens anlegen.

Dieser Rückweg wird erst entfernt, wenn DenizSideloader auf echten
Geräten nachweislich installiert, pairt und per Windows-Refresh
erneuert wurde (siehe `docs/IPADOS27_COMPAT.md`, offene Gerätetests).
