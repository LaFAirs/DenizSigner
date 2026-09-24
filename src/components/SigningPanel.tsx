import { useState } from "react";
import { useTranslation } from "react-i18next";
import { listen, emit } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { api } from "../lib/api";

export function SigningPanel({ loggedInAs, setLoggedInAs, noKeyring }: {
  loggedInAs: string | null;
  setLoggedInAs: (v: string | null) => void;
  noKeyring: boolean;
}) {
  const { t } = useTranslation();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [anisette, setAnisette] = useState("ani.sidestore.io");
  const [save, setSave] = useState(true);
  const [busy, setBusy] = useState(false);
  const [tfa, setTfa] = useState<string | null>(null);
  const [code, setCode] = useState("");

  async function signIn() {
    setBusy(true);
    try {
      const unlisten = await listen("2fa-required", () => setTfa(email));
      try {
        await api.loginNew(email, password, anisette, save);
      } finally {
        unlisten();
      }
      setPassword("");
      setLoggedInAs(email);
      toast.success(email);
    } catch (e) {
      toast.error(String((e as { message?: string })?.message ?? e));
    } finally {
      setBusy(false);
    }
  }

  if (loggedInAs) {
    return (
      <div>
        <p>{t("signing.signedInAs")}: <strong>{loggedInAs}</strong></p>
        <button
          className="btn btn-ghost"
          disabled={busy}
          onClick={() => {
            setBusy(true);
            void api
              .invalidateAccount()
              .finally(() => {
                setLoggedInAs(null);
                setBusy(false);
              });
          }}
        >
          {t("signing.signout")}
        </button>
      </div>
    );
  }

  return (
    <div>
      {noKeyring && <p className="warn">{t("signing.noKeyring")}</p>}
      <div className="field">
        <label htmlFor="email">{t("signing.email")}</label>
        <input id="email" value={email} onChange={(e) => setEmail(e.target.value)} autoComplete="username" />
      </div>
      <div className="field">
        <label htmlFor="pw">{t("signing.password")}</label>
        <input id="pw" type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="current-password" />
      </div>
      <div className="field">
        <label htmlFor="ani">{t("signing.anisette")}</label>
        <input id="ani" value={anisette} onChange={(e) => setAnisette(e.target.value)} />
      </div>
      <label className="check">
        <input type="checkbox" checked={save} onChange={(e) => setSave(e.target.checked)} />
        {t("signing.save")}
      </label>
      <div className="row">
        <button className="btn btn-primary" disabled={busy || !email || !password} onClick={() => void signIn()}>
          {t("signing.signin")}
        </button>
      </div>
      {tfa && (
        <div className="tfa">
          <p>2FA: {tfa}</p>
          <input value={code} onChange={(e) => setCode(e.target.value)} placeholder="123456" />
          <button
            className="btn btn-primary"
            onClick={() => {
              void emit("2fa-recieved", code);
              setTfa(null);
              setCode("");
            }}
          >
            OK
          </button>
        </div>
      )}
    </div>
  );
}
