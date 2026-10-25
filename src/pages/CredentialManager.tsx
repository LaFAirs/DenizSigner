import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useDialog } from "../DialogContext";
import "./CredentialManager.css";

type Certificate = { id: string; name: string; subject: string; issuer: string; serial: string; fingerprint: string; notBefore: string; expires: string; valid: boolean; selected: boolean };
type Vault = { certificates: Certificate[]; tokens: { id: string; name: string }[] };
export function CredentialManager() {
  const [vault, setVault] = useState<Vault | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [path, setPath] = useState("");
  const [label, setLabel] = useState("");
  const [password, setPassword] = useState("");
  const [tokenName, setTokenName] = useState("");
  const [token, setToken] = useState("");
  const [found, setFound] = useState<string[]>([]);
  const { confirm } = useDialog();
  async function run(work: () => Promise<void>) {
    if (busy) return;
    setBusy(true); setError(""); setMessage("");
    try { await work(); } catch (e) {
      const text = typeof e === "object" && e && "message" in e ? String(e.message) : String(e);
      setError(text);
      if (text.includes("locked")) setVault(null);
    } finally { setBusy(false); }
  }
  async function lock() {
    await invoke("credential_lock"); setVault(null); setPassword(""); setToken("");
  }
  useEffect(() => {
    invoke<string[]>("credential_discover").then(setFound).catch(() => {});
    return () => { void invoke("credential_lock"); };
  }, []);
  useEffect(() => {
    if (!vault) return;
    // The backend independently enforces expiry; this clears visible metadata too.
    let timer = window.setTimeout(() => { void lock(); }, 300000);
    const touch = () => { window.clearTimeout(timer); timer = window.setTimeout(() => { void lock(); }, 300000); };
    window.addEventListener("pointerdown", touch); window.addEventListener("keydown", touch);
    return () => { window.clearTimeout(timer); window.removeEventListener("pointerdown", touch); window.removeEventListener("keydown", touch); };
  }, [vault]);
  async function remove(id: string) {
    confirm("Eintrag entfernen", "Nur die Kopie im DenizSigner-Keyring wird entfernt. Die ursprüngliche Zertifikatsdatei bleibt erhalten. Fortfahren?", () => {
      void run(async () => setVault(await invoke<Vault>("credential_remove", { id })));
    });
  }
  async function sign() {
    const input = await open({ filters: [{ name: "IPA", extensions: ["ipa"] }] }); if (!input) return;
    const provisioning = await open({ filters: [{ name: "Provisioning Profile", extensions: ["mobileprovision"] }] }); if (!provisioning) return;
    const output = await save({ defaultPath: "signed.ipa", filters: [{ name: "IPA", extensions: ["ipa"] }] }); if (!output) return;
    await invoke("credential_sign_ipa", { input, provisioning, output });
    setMessage(`Signierte IPA gespeichert: ${output}`);
  }
  return <div className="credential-manager" aria-busy={busy}>
    <div className="credential-heading"><div><h2>Credential Manager</h2><p className="text-muted">Zertifikate · Signierprofile · Zugangsdaten</p></div>{vault && <button disabled={busy} onClick={() => void lock()}>Sperren</button>}</div>
    <p className="settings-hint">Geschützt durch den Betriebssystem-Keyring. Passwörter und private Schlüssel werden ausschließlich dort gespeichert. Automatische Sperre nach 5 Minuten Inaktivität.</p>
    {error && <p role="alert" className="credential-error">{error}</p>}
    {message && <p role="status">{message}</p>}
    {!vault ? <div className="credential-empty"><h3>Sicherer Speicher gesperrt</h3><p>Entsperren öffnet den Linux Secret Service. Der Keyring kann nach seinem Passwort fragen.</p><button disabled={busy} onClick={() => void run(async () => setVault(await invoke<Vault>("credential_unlock")))}>{busy ? "Wird entsperrt …" : "Mit Betriebssystem-Keyring entsperren"}</button></div> : <>
      <section><h3>Zertifikat importieren</h3><label>Profilname<input value={label} maxLength={200} onChange={e => setLabel(e.target.value)} placeholder="z. B. Entwicklung / Team" /></label>
        <div className="credential-file"><span>{path || "Keine Datei ausgewählt"}</span><button disabled={busy} onClick={() => void run(async () => { const file = await open({ filters: [{ name: "PKCS#12", extensions: ["p12", "pfx"] }] }); if (typeof file === "string") setPath(file); })}>.p12 / .pfx auswählen</button></div>
        {found.length > 0 && <label>Vorhandene Zertifikate<select value={path} onChange={e => setPath(e.target.value)}><option value="">Gefundene Datei auswählen …</option>{found.map(p => <option key={p} value={p}>{p}</option>)}</select></label>}
        <label>Zertifikatspasswort<input type="password" autoComplete="off" value={password} onChange={e => setPassword(e.target.value)} /></label>
        <button disabled={busy || !path || !label.trim()} onClick={() => void run(async () => { try { setVault(await invoke<Vault>("credential_import", { path, label, password })); setPath(""); setLabel(""); } finally { setPassword(""); } })}>Sicher importieren</button>
      </section>
      <section><h3>Signierprofile</h3>{vault.certificates.length === 0 && <p className="text-muted">Noch keine Zertifikate importiert.</p>}
        {vault.certificates.map(c => <article className={`credential-profile ${c.selected ? "selected" : ""}`} key={c.id}>
          <div className="credential-heading"><strong>{c.name} {c.selected && "· Ausgewählt"}</strong><span className={c.valid ? "credential-valid" : "credential-error"}>{c.valid ? "Gültig" : "Abgelaufen / noch nicht gültig"}</span></div>
          <dl><dt>Inhaber</dt><dd>{c.subject}</dd><dt>Aussteller</dt><dd>{c.issuer}</dd><dt>Gültig ab</dt><dd>{c.notBefore}</dd><dt>Ablaufdatum</dt><dd>{c.expires}</dd></dl>
          <details><summary>Zertifikatsdetails</summary><p>Seriennummer: {c.serial}</p><p>SHA-256: {c.fingerprint}</p></details>
          <div className="credential-actions"><button disabled={busy || !c.valid || c.selected} onClick={() => void run(async () => setVault(await invoke<Vault>("credential_select", { id: c.id })))}>Für Signierung auswählen</button><button disabled={busy} className="danger" onClick={() => void remove(c.id)}>Entfernen …</button></div>
        </article>)}
        <button disabled={busy || !vault.certificates.some(c => c.selected && c.valid)} onClick={() => void run(sign)}>{busy ? "Bitte warten …" : "IPA mit ausgewähltem Profil signieren …"}</button>
        <p className="settings-hint">Benötigt eine passende .mobileprovision-Datei. Exportiert eine signierte IPA. Für automatische Apple-Provisionierung und Geräteinstallation steht der bisherige Apple-ID-Workflow bereit. App-Erweiterungen benötigen eigene Provisionierungsprofile.</p>
      </section>
      <section><h3>Zugangsdaten und Tokens</h3>{vault.tokens.map(t => <div className="credential-heading" key={t.id}><span>{t.name} · ••••••••</span><button disabled={busy} onClick={() => void remove(t.id)}>Entfernen …</button></div>)}
        <label>Bezeichnung<input value={tokenName} maxLength={200} onChange={e => setTokenName(e.target.value)} /></label><label>Passwort / Token<input type="password" autoComplete="off" value={token} onChange={e => setToken(e.target.value)} /></label>
        <button disabled={busy || !tokenName.trim() || !token} onClick={() => void run(async () => { try { setVault(await invoke<Vault>("credential_save_token", { label: tokenName, value: token })); setTokenName(""); } finally { setToken(""); } })}>Im Keyring speichern</button>
      </section>
    </>}
  </div>;
}
