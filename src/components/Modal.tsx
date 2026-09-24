import { ReactNode } from "react";
import "./Modal.css";

export function Modal({ open, onClose, children, label }: {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  label: string;
}) {
  if (!open) return null;
  return (
    <div className="modal-backdrop" onClick={onClose} role="presentation">
      <div
        className="modal"
        role="dialog"
        aria-label={label}
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
