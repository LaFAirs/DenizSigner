import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { friendlyError } from "../lib/errors";

export interface FailedStep {
  stepId: string;
  extraDetails?: { type?: string; message?: string } | null;
}

export interface OperationState {
  current: string;
  started: string[];
  completed: string[];
  failed: FailedStep[];
}

interface Update {
  updateType: "started" | "finished" | "failed";
  stepId: string;
  extraDetails?: { type?: string; message?: string } | null;
}

/** Maps backend operation events onto the 7 user-facing install steps. */
const BACKEND_TO_UI: Record<string, number> = {
  prepare: 0,
  download: 0,
  read: 1,
  sign: 2,
  install: 4,
  verify: 5,
  pairing: 5,
};

export function useOperationListener(opId: string | null, onDone: (failed: boolean) => void) {
  const [state, setState] = useState<OperationState | null>(null);

  useEffect(() => {
    if (!opId) {
      setState(null);
      return;
    }
    setState({ current: opId, started: [], completed: [], failed: [] });
    let unlisten: (() => void) | null = null;
    let alive = true;
    void listen<Update>(`operation_${opId}`, (e) => {
      if (!alive) return;
      setState((old) => {
        if (!old) return old;
        if (e.payload.updateType === "started")
          return { ...old, started: [...old.started, e.payload.stepId] };
        if (e.payload.updateType === "finished") {
          const done = [...old.completed, e.payload.stepId];
          if (e.payload.stepId === "install" || e.payload.stepId === "pairing") {
            queueMicrotask(() => onDone(false));
          }
          return { ...old, completed: done };
        }
        queueMicrotask(() => onDone(true));
        return {
          ...old,
          failed: [...old.failed, { stepId: e.payload.stepId, extraDetails: e.payload.extraDetails }],
        };
      });
    }).then((f) => {
      unlisten = f;
    });
    return () => {
      alive = false;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opId]);

  return state;
}

/** Highest completed UI step index (0..6). */
export function uiProgress(state: OperationState | null): number {
  if (!state) return -1;
  if (state.failed.length > 0) {
    const f = state.failed[0];
    return BACKEND_TO_UI[f.stepId] ?? 0;
  }
  let idx = -1;
  for (const s of [...state.started, ...state.completed]) {
    const mapped = BACKEND_TO_UI[s] ?? 0;
    if (mapped > idx) idx = mapped;
  }
  if (state.completed.includes("install") || state.completed.includes("pairing")) return 6;
  return idx;
}

export function OperationFailed({ state, onClose }: { state: OperationState; onClose: () => void }) {
  const { t } = useTranslation();
  const [showTech, setShowTech] = useState(false);
  const f = state.failed[0];
  const friendly = friendlyError(f?.extraDetails?.type);
  return (
    <div className="op-failed" role="alert">
      <h3>{t("progress.failed")}</h3>
      <p className="op-title">{f?.extraDetails?.type ? friendly.title : t("progress.failed")}</p>
      <p className="hint">{friendly.hint}</p>
      <button className="link" onClick={() => setShowTech((v) => !v)}>
        {t("progress.technical")}
      </button>
      {showTech && <pre className="tech">{f?.extraDetails?.message ?? f?.stepId}</pre>}
      <div className="row">
        <button className="btn btn-ghost" onClick={onClose}>{t("progress.close")}</button>
      </div>
    </div>
  );
}
