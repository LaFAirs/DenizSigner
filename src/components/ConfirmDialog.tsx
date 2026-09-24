import { useTranslation } from "react-i18next";
import { Modal } from "./Modal";

export function ConfirmDialog({ open, title, body, confirmLabel, danger, onCancel, onConfirm }: {
  open: boolean;
  title: string;
  body: string;
  confirmLabel: string;
  danger?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  return (
    <Modal open={open} onClose={onCancel} label={title}>
      <h2>{title}</h2>
      <p className="hint">{body}</p>
      <div className="row">
        <button className="btn btn-ghost" onClick={onCancel}>{t("confirm.cancel")}</button>
        <button className={danger ? "btn btn-danger" : "btn btn-primary"} onClick={onConfirm}>
          {confirmLabel}
        </button>
      </div>
    </Modal>
  );
}
