import { useEffect } from "react";
import "./Modal.css";
import { useTranslation } from "react-i18next";

export const Modal = ({
  isOpen,
  close,
  sizeFit,
  children,
  hideClose,
  zIndex,
  closeLabel,
}: {
  children: React.ReactNode;
  isOpen: boolean;
  close?: () => void;
  sizeFit?: boolean;
  hideClose?: boolean;
  zIndex?: number;
  closeLabel?: string;
}) => {
  const { t } = useTranslation();
  useEffect(() => {
    if (!isOpen || !close) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen, close]);

  return (
    <>
      {isOpen && (
        <div
          className={`modal-container`}
          style={
            zIndex
              ? {
                  zIndex: zIndex.toString(),
                }
              : {}
          }
        >
          <div className={`modal${sizeFit ? " size-fit" : ""}`}>
            {!hideClose && close && (
              <button
                className="modal-close"
                aria-label={closeLabel ?? t("common.close")}
                onClick={() => {
                  close();
                }}
              >
                &#x2715;
              </button>
            )}
            <div className="modal-content">{children}</div>
          </div>
        </div>
      )}
    </>
  );
};
