import { useTranslation } from "react-i18next";
import { Modal } from "../components/Modal";

export function AboutDialog({ open, onClose, version }: { open: boolean; onClose: () => void; version: string }) {
  const { t } = useTranslation();
  return (
    <Modal open={open} onClose={onClose} label={t("about.title")}>
      <div className="about">
        <div className="d-badge">D</div>
        <h2>DenizSigner</h2>
        <p className="hint">{t("tagline")}</p>
        <p className="hint">v{version}</p>
        <p>{t("about.built")}</p>
        <p className="hint">{t("about.based")}</p>
        <p className="hint">{t("about.privacy")}</p>
      </div>
      <div className="row">
        <button className="btn btn-primary" onClick={onClose}>OK</button>
      </div>
    </Modal>
  );
}
