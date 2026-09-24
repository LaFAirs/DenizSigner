import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Modal } from "../components/Modal";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { api, CertificateInfo } from "../lib/api";

export function CertificatesDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const [certs, setCerts] = useState<CertificateInfo[]>([]);
  const [pending, setPending] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    void api.getCertificates().then(setCerts).catch((e) => toast.error(String(e)));
  }, [open ]);

  return (
    <Modal open={open} onClose={onClose} label={t("signing.certs")}>
      <h2>{t("signing.certs")}</h2>
      {certs.length === 0 && <p className="hint">—</p>}
      {certs.map((c) => (
        <div key={c.serialNumber ?? c.certificateId} className="cert-row">
          <div>
            <strong>{c.name ?? c.certificateId}</strong>
            <div className="hint">{c.machineName ?? ""} · {c.serialNumber ?? ""}</div>
          </div>
          <button className="btn btn-danger" onClick={() => setPending(c.serialNumber ?? "")}>
            {t("confirm.revoke")}
          </button>
        </div>
      ))}
      <div className="row">
        <button className="btn btn-primary" onClick={onClose}>OK</button>
      </div>
      <ConfirmDialog
        open={pending !== null}
        title={t("confirm.revokeTitle")}
        body={t("confirm.revokeBody")}
        confirmLabel={t("confirm.revoke")}
        danger
        onCancel={() => setPending(null)}
        onConfirm={() => {
          if (!pending) return;
          void api
            .revokeCertificate(pending)
            .then(() => {
              setCerts((old) => old.filter((c) => c.serialNumber !== pending));
              setPending(null);
            })
            .catch((e) => toast.error(String(e)));
        }}
      />
    </Modal>
  );
}
