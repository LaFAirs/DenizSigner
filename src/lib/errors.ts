/** Friendly error mapping (mirrors backend `friendly()`). */
const MAP: Record<string, { title: string; hint: string }> = {
  max_apps: {
    title: "Installation failed — app limit reached",
    hint: "This Apple ID already has the maximum number of sideloaded apps. Remove an unused app from the device, then try again.",
  },
  not_enough_app_ids: {
    title: "Installation failed — no free App ID",
    hint: "Apple rejected the provisioning profile. Free up an App ID (max 10 per 7 days on free accounts) or wait.",
  },
  account_locked: {
    title: "Apple account locked",
    hint: "Unlock it at iforgot.apple.com, then sign in again.",
  },
  auth: {
    title: "Sign-in failed",
    hint: "Check your Apple ID, password and 2FA code.",
  },
  developer: {
    title: "Apple Developer request rejected",
    hint: "Check your Apple Developer account, device pairing and Developer Mode.",
  },
  underage: {
    title: "Apple Developer request rejected",
    hint: "Check your Apple Developer account, device pairing and Developer Mode.",
  },
  anisette: {
    title: "Apple verification service unreachable",
    hint: "The configured Anisette server did not answer. Check Settings → Signing.",
  },
  network: {
    title: "Blocked by network policy",
    hint: "This request is outside the documented allowlist (Settings → Privacy).",
  },
  invalid_ipa: {
    title: "Invalid IPA file",
    hint: "The file is not a valid .ipa (a ZIP containing Payload/*.app). Choose another file.",
  },
  not_logged_in: { title: "Not signed in", hint: "Sign in with your Apple ID first." },
  no_device_selected: {
    title: "No device selected",
    hint: "Connect your iPhone/iPad via USB and select it.",
  },
};

export function friendlyError(type?: string): { title: string; hint: string } {
  if (type && MAP[type]) return MAP[type];
  if (type === "device_coms" || type === "device_coms_with_message" || type === "usbmuxd")
    return {
      title: "Device communication failed",
      hint: "Check USB, trust prompt, and iTunes/Apple Devices on Windows.",
    };
  if (type === "house_arrest")
    return {
      title: "Pairing file could not be placed",
      hint: "Make sure the target app is installed and opened once, then retry.",
    };
  if (type === "remote_pairing" || type === "lockdown_pairing")
    return {
      title: "Pairing failed",
      hint: "Tap Trust on the device when prompted, keep it unlocked, then retry.",
    };
  return {
    title: "Something went wrong",
    hint: "Check your Apple Developer account, device pairing and Developer Mode.",
  };
}

/** Client-side guard: never send secrets to logs. */
export function sanitizeForLog(message: string): string {
  return message
    .replace(/(password|token|2fa|secret|cookie|session)\s*[:=]\s*\S+/gi, "$1=[redacted]")
    .slice(0, 2000);
}
