# Install DenizSideloader (Windows-Workflow)

Ersetzt den bisherigen Menüpunkt „Install SideStore“. SideStore bleibt
als dokumentierter Rückweg erhalten (`docs/RECOVERY_SIDESTORE.md`),
wird in der UI aber nicht mehr angeboten.

Voraussetzungen: Windows 10 (1809+) / 11 x64, Apple-Gerätetreiber
(Apple Devices oder iTunes), USB-Kabel, Apple-ID (kostenlos reicht),
`DenizSideloader-unsigned.ipa` aus dem iOS-CI-Artefakt oder lokalem
Xcode-Build (`DenizSideloader/README.md`).

1. **iPad per USB verbinden.** Gerät entsperren, **Vertrauen** tippen,
   ggf. Code eingeben.
2. **Gerät erkennen.** In DenizSigner erscheint es in der Gerätekarte;
   bei Problemen: anderer Port/Kabel, Apple Devices prüfen, Refresh.
3. **DenizSideloader-IPA bereitstellen.** Datei aus dem CI-Artefakt
   (`DenizSideloader-unsigned`) herunterladen und ablegen — kein
   Download in der App, keine neuen Netzwerkziele.
4. **Mit Apple-ID signieren.** **Install DenizSideloader** klicken, IPA
   wählen, Apple-ID + 2FA eingeben (2FA-Fenster öffnet sich automatisch).
   Passwort nur bei Opt-in im OS-Schlüsselbund gespeichert.
5. **Auf dem iPad installieren.** DenizSigner signiert per `isideload`
   und installiert per `idevice`-Transport; Fortschritt: Prüfen →
   Installieren → Pairing.
6. **Pairing-Daten übertragen.** Läuft automatisch (AFC in den
   Documents-Ordner von DenizSideloader). DenizSideloader danach einmal
   öffnen.
7. **Kompatibilität und Installation prüfen.** In DenizSideloader:
   Pairing „vorhanden“, verwaltete App mit Ablaufdatum (~+7 Tage bei
   kostenloser Apple-ID). Detailprüfung: Apps → App → Diagnostizieren.
8. **Erfolgreiche Einrichtung.** Refresh-Tab zeigt die Fälligkeit;
   Erinnerungen aktivieren. Fertig.

Fehlerfälle: Technische Details stehen unter jedem Schritt
(„View technical details“); Logs: Einstellungen → View Logs. Klare
Fälle: 3-App-Limit (kostenlos) → ungenutzte Apps löschen; keine freie
App-ID → 7-Tage-Fenster abwarten; Pairing schlägt fehl → entsperrt
lassen, Vertrauen erneut tippen.
