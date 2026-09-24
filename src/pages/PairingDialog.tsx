import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Modal } from "../components/Modal";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { api } from "../lib/api";

interface PairingApp {
  name: string;
  bundleId: string;
  path: string;
}

export function PairingDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const [apps, setApps] = useState<PairingApp[]>([]);
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    if (!open) return;
    void api.installedPairingApps().then(setApps).catch((e) => toast.error(String(e)));
  }, [open ]);

  return (
    <Modal open={open} onClose={onClose} label={t("pairing.title")}>
      <h2>{t("pairing.title")}</h2>
      {apps.length === 0 && <p className="hint">{t("pairing.none")}</p>}
      {apps.map((a) => (
        <div key={a.bundleId} className="cert-row">
          <strong>{a.name}</strong>
          <button
            className="btn btn-primary"
            onClick={() => {
              void api.placePairing(a.bundleId, a.path).catch((e) => toast.error(String(e)));
            }}
          >
            {t("pairing.place")}
          </button>
        </div>
      ))}
      <div className="row start">
        <button className="btn btn-ghost" onClick={() => void api.exportPairing().catch((e) => toast.error(String(e)))}>
          {t("pairing.export")}
        </button>
        <button className="btn btn-ghost" onClick={() => setConfirmDelete(true)}>
          {t("pairing.delete")}
        </button>
      </div>
      <div className="row">
        <button className="btn btn-primary" onClick={onClose}>OK</button>
      </div>
      <ConfirmDialog
        open={confirmDelete}
        title={t("confirm.deletePairingTitle")}
        body={t("confirm.deletePairingBody")}
        confirmLabel={t("confirm.delete")}
        danger
        onCancel={() => setConfirmDelete(false)}
        onConfirm={() => {
          void api
            .deleteStoredRppairing()
            .then(() => setConfirmDelete(false))
            .catch((e) => toast.error(String(e)));
        }}
      />
    </Modal>
  );
}
