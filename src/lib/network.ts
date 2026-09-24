/** Documented network allowlist (mirrors backend `network_allowlist.rs`). */
export const ALLOWED_SERVICES: Array<{ host: string; purpose: string; data: string }> = [
  {
    host: "Apple (via isideload)",
    purpose: "Apple-ID auth, certificates, App IDs, provisioning, signing",
    data: "Apple-ID + 2FA (session only), CSR/cert requests, provisioning profiles",
  },
  {
    host: "Configured Anisette server (default ani.sidestore.io)",
    purpose: "Apple auth helper (Anisette headers)",
    data: "Device-routing identifiers required by Apple's auth protocol",
  },
  {
    host: "github.com (+ release asset hosts)",
    purpose: "Only for IPAs you explicitly install (SideStore / LiveContainer)",
    data: "Requested release artifact bytes",
  },
  {
    host: "iforgot.apple.com, apple.co",
    purpose: "Help links opened in the system browser",
    data: "Nothing automatically — browser navigation only",
  },
];

export function isIpaPath(path: string): boolean {
  return path.toLowerCase().endsWith(".ipa");
}
