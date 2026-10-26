export const DEFAULT_ANISETTE_SERVER = "mac-ani.jkcoxson.com";
export const anisetteServers = [
  [DEFAULT_ANISETTE_SERVER, "Jkcoxson (Mac)"],
  ["new.ani.sidestore.io", "SideStore (Mac, .io)"],
  ["ani3.stikstore.app", "StikStore (Mac)"],
];

const legacyServers = new Set([
  "ani.sidestore.io", "ani.stikstore.app", "ani.sidestore.app",
  "ani.sidestore.zip", "ani.846969.xyz", "ani.neoarz.xyz",
  "ani.xu30.top", "anisette.wedotstud.io",
]);

export function migrateAnisetteServer(value: unknown): string {
  if (typeof value !== "string" || !value.trim()) return DEFAULT_ANISETTE_SERVER;
  try {
    const url = new URL(value.includes("://") ? value : `https://${value}`);
    if (url.protocol === "https:" && !url.username && !url.password &&
        !url.port && url.pathname === "/" && !url.search && !url.hash &&
        legacyServers.has(url.hostname)) return DEFAULT_ANISETTE_SERVER;
  } catch { /* Leave custom values to backend URL validation. */ }
  return value;
}
