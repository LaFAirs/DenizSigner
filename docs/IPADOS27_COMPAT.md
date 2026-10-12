# iPadOS-27-Kompatibilität (DenizSideloader)

Stand: 12.10.2026. iPadOS 27 ist zum jetzigen Zeitpunkt **nicht
öffentlich verfügbar** (aktuell: iPadOS 26.x). Alles unter „27“ ist
daher Prognose aus öffentlichen Apple-Dokumentationen und dem Verhalten
von iPadOS 17–26 — **ungetestet**, bis ein 27er-Gerät geprüft ist.

## Machbarkeitsmatrix

| Funktion | Wo | iPadOS 26 (erwartet) | iPadOS 27 (Prognose) | Status |
|---|---|---|---|---|
| IPA-Metadaten lesen (ZIP + Info.plist) | on-device | funktioniert (reine Dateiarbeit) | keine Änderung zu erwarten | implementiert, Unit-getestet |
| Ablaufdaten tracken, erinnern (lokale Mitteilungen) | on-device | funktioniert | keine Änderung zu erwarten | implementiert, Unit-getestet |
| Pairing-Datei prüfen/importieren (Documents) | on-device | funktioniert (Files + File Sharing) | keine Änderung zu erwarten | implementiert, Unit-getestet |
| Diagnose, Logs, Keychain | on-device | funktioniert | keine Änderung zu erwarten | implementiert, Unit-getestet |
| Apple-ID-Login mit 2FA | Windows (DenizSigner) | funktioniert (`isideload` + Anisette) | von Apple-Server abhängig, nicht von iPadOS | bestehend, erhalten |
| Zertifikate / App-IDs / Profile | Windows (DenizSigner) | funktioniert | dto. | bestehend, erhalten |
| Signieren + Installation per USB | Windows (DenizSigner) | funktioniert (Lockdown/AFC/HouseArrest, RSD-Tunnel ab 17.4) | **Kernrisiko**: Apple kann USB-Protokolle (Lockdown, CoreDevice-Tunnel, HouseArrest) jederzeit ändern; seit iOS 17 ist Pairing restriktiver geworden | implementiert (bestehender Transport), **Gerätetest ausstehend** |
| Pairing-Datei ablegen (AFC in App-Documents) | Windows → Gerät | funktioniert, App muss einmal geöffnet worden sein | dto. | implementiert, **Gerätetest ausstehend** |
| On-device Installation ohne PC | on-device | **nicht möglich** (Sandbox, keine Installations-API für Drittanbieter) | keine Änderung zu erwarten | bewusst NICHT implementiert (`DeviceTransport` wirft) |
| On-device Re-Signing ohne PC | on-device | **nicht möglich** (kein Codesigning-Zugang, keine privaten Profile) | keine Änderung zu erwarten | bewusst NICHT implementiert |
| Garantierte Hintergrund-Erneuerung | on-device | **nicht möglich** (`BGAppRefreshTask` best-effort) | keine Änderung zu erwarten | nicht behauptet; Erinnerungen stattdessen |

## Windows-gestützter Fallback (funktionsfähig)

Weil On-device-Signierung/Installation unter iPadOS 27 nicht seriös
versprechbar ist, ist der Fallback **der Hauptweg**, kein Notnagel:

1. USB-Verbindung, Pairing und Transport: bestehender DenizSigner-Code
   (`idevice`/`isideload`), erweitert um `install_denizsideloader`.
2. Ablauf-Tracking und Erinnerungen: on-device (DenizSideloader).
3. Erneuerung: per USB in DenizSigner, danach „Als erneuert markieren“.

## Offene Gerätetests (alle als ungetestet markiert)

- [ ] Installation + Pairing-Ablage auf echtem iPad (26.x)
- [ ] Erneuter Test nach Erscheinen von iPadOS 27 (USB-Protokolle)
- [ ] Erinnerungs-Mitteilungen im Alltag (Zustellung vor Ablauf)
- [ ] IPA-Parser gegen reale Store-/Build-IPAs (deflate-Varianten)
