import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, DeviceInfo } from "../lib/api";

export function DeviceCard({ selected, onSelect }: {
  selected: DeviceInfo | null;
  onSelect: (d: DeviceInfo | null) => void;
}) {
  const { t } = useTranslation();
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const list = await api.listDevices();
      const ok = list
        .map((r) => ("Ok" in r ? (r as { Ok: DeviceInfo }).Ok : null))
        .filter((d): d is DeviceInfo => d !== null);
      setDevices(ok);
      if (ok.length === 0) onSelect(null);
    } catch {
      setDevices([]);
    } finally {
      setBusy(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function pick(d: DeviceInfo) {
    setBusy(true);
    try {
      await api.setSelectedDevice(d);
      onSelect(d);
    } catch {
      onSelect(null);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-label={t("device.statusLabel")} className="device-card">
      {selected ? (
        <div className="device-on">
          <span className="dot dot-on" aria-hidden="true" />
          <div>
            <strong>
              {selected.name} {t("device.connected")}
            </strong>
            <div className="hint">
              iOS {selected.version} · {selected.connectionType === "USB" ? t("device.viaUsb") : selected.connectionType}
            </div>
          </div>
        </div>
      ) : (
        <div className="device-off">
          <span className="dot dot-off" aria-hidden="true" />
          <strong>{t("device.none")}</strong>
        </div>
      )}
      <div className="device-row">
        <select
          aria-label={t("device.select")}
          value={selected?.udid ?? ""}
          disabled={busy || devices.length === 0}
          onChange={(e) => {
            const d = devices.find((x) => x.udid === e.target.value);
            if (d) void pick(d);
          }}
        >
          <option value="">{t("device.select")}</option>
          {devices.map((d) => (
            <option key={d.udid} value={d.udid}>
              {d.name} (iOS {d.version})
            </option>
          ))}
        </select>
        <button className="btn btn-ghost" disabled={busy} onClick={() => void refresh()}>
          {t("device.refresh")}
        </button>
      </div>
    </section>
  );
}
