import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import "./App.css";
import { api, DeviceInfo } from "./lib/api";
import { isIpaPath } from "./lib/network";
import { DeviceCard } from "./components/DeviceCard";
import { OperationFailed, uiProgress, useOperationListener } from "./components/operations";
import { SettingsDialog, Prefs } from "./pages/SettingsDialog";
import { LogsDialog } from "./pages/LogsDialog";
import { AboutDialog } from "./pages/AboutDialog";
import { CertificatesDialog } from "./pages/CertificatesDialog";
import { PairingDialog } from "./pages/PairingDialog";

const DEFAULT_PREFS: Prefs = {
  lang: "en",
  theme: "blue",
  notifications: true,
  startMinimized: false,
  anisette: "ani.sidestore.io",
  debugLogs: false,
};

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem("denizsigner.prefs");
    if (raw) return { ...DEFAULT_PREFS, ...JSON.parse(raw) };
  } catch {
    /* ignore */
  }
  return DEFAULT_PREFS;
}

export default function App() {
  const { t, i18n } = useTranslation();
  const [version, setVersion] = useState("0.1.0");
  const [loggedInAs, setLoggedInAs] = useState<string | null>(null);
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [noKeyring, setNoKeyring] = useState(false);
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  const [dialog, setDialog] = useState<null | "settings" | "logs" | "about" | "certs" | "pairing">(null);
  const [ipaPath, setIpaPath] = useState<string | null>(null);
  const [opId, setOpId] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [installing, setInstalling] = useState(false);
  const deviceRef = useRef<HTMLElement | null>(null);

  const opState = useOperationListener(opId, () => setInstalling(false));

  useEffect(() => {
    void getVersion().then(setVersion).catch(() => {});
    void api.loggedInAs().then(setLoggedInAs).catch(() => {});
    void api.keyringAvailable().then((ok) => setNoKeyring(!ok)).catch(() => setNoKeyring(true));
    void i18n.changeLanguage(loadPrefs().lang).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Native file drop (Tauri webview): payload.paths
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void listen<{ paths: string[] }>("tauri://drag-drop", (e) => {
      const p = e.payload.paths?.[0];
      if (p) {
        if (!isIpaPath(p)) {
          toast.error(t("drop.title") + ": .ipa");
          return;
        }
        setIpaPath(p);
      }
      setDragOver(false);
    }).then((f) => {
      unlisten = f;
    });
    return () => unlisten?.();
  }, [t]);

  const chooseIpa = useCallback(async () => {
    const path = await openFileDialog({
      multiple: false,
      filters: [{ name: "IPA", extensions: ["ipa"] }],
    });
    if (typeof path === "string") setIpaPath(path);
  }, []);

  async function install() {
    if (!loggedInAs) {
      toast.error(t("drop.needLogin"));
      setDialog("settings");
      return;
    }
    if (!device) {
      toast.error(t("drop.needDevice"));
      deviceRef.current?.scrollIntoView({ behavior: "smooth" });
      return;
    }
    if (!ipaPath) {
      await chooseIpa();
      return;
    }
    setOpId(null);
    setInstalling(true);
    setOpId("sideload");
    try {
      await api.sideload(ipaPath);
    } catch {
      // Failure is delivered via operation_failed event; keep panel open.
    }
  }

  const progress = uiProgress(opState);
  const steps: string[] = t("progress.steps", { returnObjects: true }) as unknown as string[];
  const failed = opState && opState.failed.length > 0;

  return (
    <main className={`shell theme-${prefs.theme}`}>
      <header className="topbar">
        <button className="brand" onClick={() => setDialog("about")} aria-label="DenizSigner — About">
          <span className="d-badge">D</span>
          <span className="brand-text">
            <strong>DenizSigner</strong>
            <small>{t("subtitle")}</small>
          </span>
        </button>
        <nav className="top-actions">
          <button className="top-btn" onClick={() => deviceRef.current?.scrollIntoView({ behavior: "smooth" })}>
            {t("header.device")}
          </button>
          <button className="top-btn" onClick={() => setDialog("logs")}>{t("header.logs")}</button>
          <button className="top-btn" onClick={() => setDialog("settings")}>{t("header.settings")}</button>
        </nav>
      </header>

      <div className="content">
        <section ref={deviceRef} className="card device-section">
          <DeviceCard selected={device} onSelect={setDevice} />
          <div className="quick-links">
            <button className="link" onClick={() => (loggedInAs ? setDialog("certs") : setDialog("settings"))}>
              {t("signing.certs")}
            </button>
            <button className="link" onClick={() => (device ? setDialog("pairing") : toast.error(t("drop.needDevice")))}>
              {t("pairing.title")}
            </button>
          </div>
        </section>

        <section
          className={`card drop ${dragOver ? "drag" : ""}`}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={(e) => {
            e.preventDefault();
            setDragOver(false);
            const f = e.dataTransfer.files?.[0] as unknown as { path?: string; name?: string } | undefined;
            const p = f?.path ?? (f?.name && isIpaPath(f.name) ? f.name : undefined);
            if (p && isIpaPath(p)) setIpaPath(p);
            else void chooseIpa();
          }}
        >
          <h2>{t("drop.title")}</h2>
          <p className="hint">{t("drop.subtitle")}</p>
          {ipaPath && <p className="ipa-path" title={ipaPath}>{ipaPath}</p>}
          <div className="drop-actions">
            <button className="btn btn-ghost" onClick={() => void chooseIpa()}>{t("drop.choose")}</button>
            <button className="btn btn-primary" disabled={installing} onClick={() => void install()}>
              {t("drop.install")}
            </button>
          </div>
          {installing || opState ? (
            <div className="progress">
              <div className="bar">
                <div className="fill" style={{ width: `${Math.max(0, Math.min(6, progress)) / 6 * 100}%` }} />
              </div>
              <ol className="steps">
                {steps.map((s, i) => (
                  <li key={s} className={i <= progress ? "done" : ""} aria-current={i === progress ? "step" : undefined}>
                    {i + 1}. {s}
                  </li>
                ))}
              </ol>
              {!installing && !failed && progress >= 6 && <p className="ok">{t("progress.finished")}</p>}
              {failed && opState && (
                <OperationFailed state={opState} onClose={() => setOpId(null)} />
              )}
            </div>
          ) : (
            <p className="ready">{loggedInAs && device ? t("drop.ready") : t("tagline")}</p>
          )}
        </section>
      </div>

      <SettingsDialog
        open={dialog === "settings"}
        onClose={() => setDialog(null)}
        prefs={prefs}
        setPrefs={setPrefs}
        loggedInAs={loggedInAs}
        setLoggedInAs={setLoggedInAs}
        noKeyring={noKeyring}
        version={version}
      />
      <LogsDialog open={dialog === "logs"} onClose={() => setDialog(null)} />
      <AboutDialog open={dialog === "about"} onClose={() => setDialog(null)} version={version} />
      <CertificatesDialog open={dialog === "certs"} onClose={() => setDialog(null)} />
      <PairingDialog open={dialog === "pairing"} onClose={() => setDialog(null)} />
    </main>
  );
}
