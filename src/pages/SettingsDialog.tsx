import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Modal } from "../components/Modal";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { ALLOWED_SERVICES } from "../lib/network";
import { api } from "../lib/api";
import { SigningPanel } from "../components/SigningPanel";

export interface Prefs {
  lang: string;
  theme: string;
  notifications: boolean;
  startMinimized: boolean;
  anisette: string;
  debugLogs: boolean;
}

export function SettingsDialog({ open, onClose, prefs, setPrefs, loggedInAs, setLoggedInAs, noKeyring, version }: {
  open: boolean;
  onClose: () => void;
  prefs: Prefs;
  setPrefs: (p: Prefs) => void;
  loggedInAs: string | null;
  setLoggedInAs: (v: string | null) => void;
  noKeyring: boolean;
  version: string;
}) {
  const { t, i18n } = useTranslation();
  const [confirmReset, setConfirmReset] = useState(false);

  function set<K extends keyof Prefs>(k: K, v: Prefs[K]) {
    const next = { ...prefs, [k]: v };
    setPrefs(next);
    try {
      localStorage.setItem("denizsigner.prefs", JSON.stringify(next));
    } catch {
      /* local-only, ignore */
    }
    if (k === "lang") void i18n.changeLanguage(String(v));
    if (k === "debugLogs") void api.setLogLevelDebug(Boolean(v));
  }

  return (
    <Modal open={open} onClose={onClose} label={t("settings.title")}>
      <h2>{t("settings.title")}</h2>

      <h3>{t("settings.general")}</h3>
      <div className="field">
        <label htmlFor="lang">{t("settings.language")}</label>
        <select id="lang" value={prefs.lang} onChange={(e) => set("lang", e.target.value)}>
          <option value="en">English</option>
          <option value="de">Deutsch</option>
        </select>
      </div>
      <div className="field">
        <label htmlFor="theme">{t("settings.theme")}</label>
        <select id="theme" value={prefs.theme} onChange={(e) => set("theme", e.target.value)}>
          <option value="blue">DenizSigner Blue</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </div>
      <label className="check">
        <input type="checkbox" checked={prefs.notifications} onChange={(e) => set("notifications", e.target.checked)} />
        {t("settings.notifications")}
      </label>
      <label className="check">
        <input type="checkbox" checked={prefs.startMinimized} onChange={(e) => set("startMinimized", e.target.checked)} />
        {t("settings.startMinimized")}
      </label>

      <h3>{t("settings.signing")}</h3>
      <SigningPanel loggedInAs={loggedInAs} setLoggedInAs={setLoggedInAs} noKeyring={noKeyring} />

      <h3>{t("settings.privacy")}</h3>
      <p className="hint">{t("settings.privacyNote")}</p>
      <h4>{t("settings.networkActivity")}</h4>
      <ul className="allowlist">
        {ALLOWED_SERVICES.map((s) => (
          <li key={s.host}>
            <strong>{s.host}</strong>
            <div className="hint">{s.purpose}</div>
            <div className="hint">Data: {s.data}</div>
          </li>
        ))}
      </ul>

      <h3>{t("settings.advanced")}</h3>
      <label className="check">
        <input type="checkbox" checked={prefs.debugLogs} onChange={(e) => set("debugLogs", e.target.checked)} />
        {t("settings.debugLogs")} ({t("settings.logLevel")}: {prefs.debugLogs ? "Debug" : "Info"})
      </label>
      <div className="row start">
        <button className="btn btn-ghost" onClick={() => setConfirmReset(true)}>{t("settings.reset")}</button>
      </div>
      <p className="hint">DenizSigner v{version} · local-first · no accounts · no telemetry</p>

      <div className="row">
        <button className="btn btn-primary" onClick={onClose}>{t("settings.close")}</button>
      </div>

      <ConfirmDialog
        open={confirmReset}
        title={t("confirm.resetTitle")}
        body={t("confirm.resetBody")}
        confirmLabel={t("settings.reset")}
        danger
        onCancel={() => setConfirmReset(false)}
        onConfirm={() => {
          try {
            localStorage.removeItem("denizsigner.prefs");
          } catch {
            /* ignore */
          }
          setConfirmReset(false);
          onClose();
        }}
      />
    </Modal>
  );
}
