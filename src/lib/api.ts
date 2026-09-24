import { invoke } from "@tauri-apps/api/core";

export interface DeviceInfo {
  name: string;
  id: number;
  udid: string;
  connectionType: string;
  version: string;
}

export interface AppErrorShape {
  type: string;
  message: string;
}

export interface CertificateInfo {
  name?: string;
  certificateId?: string;
  serialNumber?: string;
  machineName?: string;
  machineId?: string;
}

export const api = {
  listDevices: () => invoke<Array<{ Ok?: DeviceInfo; Err?: AppErrorShape }>>("list_devices"),
  setSelectedDevice: (device: DeviceInfo | null) =>
    invoke<void>("set_selected_device", { device }),
  cancelPairing: () => invoke<void>("cancel_pairing"),
  sideload: (appPath: string) => invoke<void>("sideload_operation", { appPath }),
  installSideStore: (nightly: boolean, liveContainer: boolean) =>
    invoke<void>("install_sidestore_operation", { nightly, liveContainer }),
  loginNew: (email: string, password: string, anisetteServer: string, saveCredentials: boolean) =>
    invoke<void>("login_new", { email, password, anisetteServer, saveCredentials }),
  loginStored: (email: string, anisetteServer: string) =>
    invoke<void>("login_stored", { email, anisetteServer }),
  deleteAccount: (email: string) => invoke<void>("delete_account", { email }),
  loggedInAs: () => invoke<string | null>("logged_in_as"),
  invalidateAccount: () => invoke<void>("invalidate_account"),
  resetAnisetteState: () => invoke<boolean>("reset_anisette_state"),
  getCertificates: () => invoke<CertificateInfo[]>("get_certificates"),
  revokeCertificate: (serialNumber: string) =>
    invoke<void>("revoke_certificate", { serialNumber }),
  listAppIds: () => invoke<unknown>("list_app_ids"),
  deleteAppId: (appIdId: string) => invoke<void>("delete_app_id", { appIdId }),
  placePairing: (bundleId: string, path: string) =>
    invoke<void>("place_pairing_cmd", { bundleId, path }),
  exportPairing: () => invoke<void>("export_pairing_cmd"),
  deleteStoredRppairing: () => invoke<void>("delete_stored_rppairing"),
  hasStoredRppairing: (device: DeviceInfo) =>
    invoke<boolean>("has_stored_rppairing", { device }),
  installedPairingApps: () =>
    invoke<Array<{ name: string; bundleId: string; path: string }>>("installed_pairing_apps"),
  keyringAvailable: () => invoke<boolean>("keyring_available"),
  forceDisableKeyring: (force: boolean) =>
    invoke<void>("force_disable_keyring", { force }),
  setLogLevelDebug: (enabled: boolean) =>
    invoke<void>("set_log_level_debug", { enabled }),
};
