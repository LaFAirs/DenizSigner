import { useTranslation } from "react-i18next";
import { Modal } from "../components/Modal";
import { useLogs } from "../components/LogContext";

export function LogsDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const { logs, clear } = useLogs();
  return (
    <Modal open={open} onClose={onClose} label={t("logs.title")}>
      <h2>{t("logs.title")}</h2>
      {logs.length === 0 ? (
        <p className="hint">{t("logs.empty")}</p>
      ) : (
        <pre className="logs">
          {logs
            .map(
              (l) =>
                `[${l.timestamp}] [${["?", "TRACE", "DEBUG", "INFO", "WARN", "ERROR"][l.level] ?? l.level}] ${l.message}`,
            )
            .join("\n")}
        </pre>
      )}
      <div className="row">
        <button className="btn btn-ghost" onClick={clear}>{t("logs.clear")}</button>
        <button
          className="btn btn-ghost"
          onClick={() => {
            const text = logs.map((l) => `[${l.timestamp}] ${l.message}`).join("\n");
            void navigator.clipboard?.writeText(text);
          }}
        >
          {t("logs.export")}
        </button>
        <button className="btn btn-primary" onClick={onClose}>{t("logs.close")}</button>
      </div>
    </Modal>
  );
}
