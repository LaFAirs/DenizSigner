import "./Certificates.css";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useError } from "../ErrorContext";
import { useDialog } from "../DialogContext";
import { useTranslation } from "react-i18next";

export type Certificate = {
  name: string;
  certificateId: string;
  serialNumber: string;
  machineName: string;
  machineId: string;
};

export const Certificates = () => {
  const { t } = useTranslation();
  const [certificates, setCertificates] = useState<Certificate[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const loadingRef = useRef<boolean>(false);
  const { err } = useError();
  const { confirm } = useDialog();

  const loadCertificates = useCallback(async () => {
    if (loadingRef.current) return;
    const promise = async () => {
      loadingRef.current = true;
      setLoading(true);
      let certs = await invoke<Certificate[]>("get_certificates");
      setCertificates(certs);
      setLoading(false);
      loadingRef.current = false;
    };
    toast.promise(promise, {
      loading: t("certificates.loading"),
      success: t("certificates.loaded_success"),
      error: (e) => err(t("certificates.failed_load"), e),
    });
  }, [setCertificates, t]);

  const revokeCertificate = useCallback(
    async (serialNumber: string) => {
      const promise = invoke<void>("revoke_certificate", {
        serialNumber,
      });
      promise.then(loadCertificates);
      toast.promise(promise, {
        loading: t("certificates.revoking"),
        success: t("certificates.revoked_success"),
        error: (e) => err(t("certificates.failed_revoke"), e),
      });
    },
    [setCertificates, loadCertificates, t],
  );

  useEffect(() => {
    loadCertificates();
  }, []);

  return (
    <>
      <h2>{t("certificates.manage")}</h2>
      {certificates.length === 0 ? (
        <div>{loading ? t("certificates.loading") : t("certificates.none_found")}</div>
      ) : (
        <div className="card">
          <div className="certificate-table-container">
            <table className="certificate-table">
              <thead>
                <tr className="certificate-item">
                  <th className="cert-item-part">{t("certificates.name")}</th>
                  <th className="cert-item-part">{t("certificates.serial_number")}</th>
                  <th className="cert-item-part">{t("certificates.machine_name")}</th>
                  <th className="cert-item-part">{t("certificates.machine_id")}</th>
                  <th>{t("certificates.revoke")}</th>
                </tr>
              </thead>
              <tbody>
                {certificates.map((cert, i) => (
                  <tr
                    key={cert.certificateId}
                    className={
                      "certificate-item" +
                      (i === certificates.length - 1 ? " cert-item-last" : "")
                    }
                  >
                    <td className="cert-item-part">{cert.name}</td>
                    <td className="cert-item-part">{cert.serialNumber}</td>
                    <td className="cert-item-part">{cert.machineName}</td>
                    <td className="cert-item-part">{cert.machineId}</td>
                    <td
                      className="cert-item-revoke"
                      role="button"
                      tabIndex={0}
                      aria-label={t("certificates.revoke")}
                      onClick={() =>
                        confirm(
                          t("certificates.revoke_title"),
                          t("certificates.revoke_message"),
                          () => revokeCertificate(cert.serialNumber),
                        )
                      }
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          (e.currentTarget as HTMLElement).click();
                        }
                      }}
                    >
                      {t("certificates.revoke")}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
      <button
        style={{ marginTop: "1em", width: "100%" }}
        onClick={loadCertificates}
        disabled={loading}
      >
        {t("common.refresh")}
      </button>
    </>
  );
};
