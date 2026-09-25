import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");
let n = 0;
const ok = (cond, name) => { assert.ok(cond, name); n++; console.log(`  ok ${n} - ${name}`); };

// 1. iloader-exact layout present with DenizSigner branding.
for (const f of ["src/main.tsx", "src/App.tsx", "src/AppleID.tsx", "src/Device.tsx",
  "src/errors.tsx", "src/ErrorContext.tsx", "src/LogContext.tsx", "src/StoreContext.tsx",
  "src/PlatformContext.tsx", "src/DialogContext.tsx", "src/i18next.ts",
  "src/components/Modal.tsx", "src/components/GlassCard.tsx", "src/components/Dropdown.tsx",
  "src/components/operations.ts", "src/components/OperationView.tsx",
  "src/pages/Settings.tsx", "src/pages/Certificates.tsx", "src/pages/Pairing.tsx",
  "src/pages/AppIds.tsx", "src/App.css"]) {
  ok(existsSync(join(root, f)), `layout file: ${f}`);
}
ok(read("index.html").includes("<title>DenizSigner</title>"), "window title DenizSigner");
ok(read("src/App.tsx").includes("DenizSigner"), "brand in header");
ok(!read("src/App.tsx").toLowerCase().includes("iloader"), "no upstream name in App.tsx");

// 2. No updater wiring in frontend.
ok(!read("src/App.tsx").includes("checkForUpdates"), "no update checker");
ok(!existsSync(join(root, "src/update.ts")), "no update.ts");

// 3. Backend boundary + IPA validation + redaction intact.
ok(read("src-tauri/src/network_allowlist.rs").includes("require_allowed_url"), "backend boundary");
ok(read("src-tauri/src/ipa.rs").includes("Payload/"), "backend Payload check");
ok(read("src-tauri/src/logging.rs").includes("redact"), "log redaction");
ok(read("src-tauri/src/secure_storage.rs").includes('"denizsigner"'), "keyring service denizsigner");

// 4. No telemetry / updater / upstream endpoints in shipped code.
const shipped = ["src/App.tsx", "src/AppleID.tsx", "src/Device.tsx", "src/errors.tsx",
  "src/pages/Settings.tsx", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml",
  "package.json", "index.html"].map(read).join("\n").toLowerCase();
for (const bad of ["telemetry", "sentry", "posthog", "plausible", "tauri-plugin-updater",
  "latest.json", "nab138", "iloader", "me.nabdev", "umami", "amplitude", "hotjar",
  "discord.gg", "crashlytics", "bugsnag"]) {
  ok(!shipped.includes(bad), `no forbidden token: ${bad}`);
}

// 5. Privacy section present in Settings; locales carry it.
ok(read("src/pages/Settings.tsx").includes("privacy_note"), "privacy block in Settings");
ok(read("src/locales/en.json").includes("privacy_note"), "en privacy keys");
ok(read("src/locales/de.json").includes("privacy_note"), "de privacy keys");

// 6. Confirm dialog (DialogContext) used for destructive actions.
ok(read("src/DialogContext.tsx").includes("confirm"), "confirm dialog");
ok(read("src/pages/Settings.tsx").includes("confirm("), "confirm used in Settings");
ok(read("src/pages/Pairing.tsx").includes("confirm("), "confirm used in Pairing");
ok(read("src/pages/Certificates.tsx").includes("revoke_title"), "revoke confirm in Certificates");
ok(read("src/pages/AppIds.tsx").includes("delete_title"), "delete confirm in AppIds");

// 7. Secrets hygiene in frontend: password cleared after login, never stored.
const appleId = read("src/AppleID.tsx");
ok(appleId.includes('setPasswordInput("")'), "password cleared from UI memory");
ok(!appleId.includes("localStorage"), "no localStorage in AppleID");
ok(!appleId.includes("store.set"), "no store writes in AppleID");
ok(!appleId.includes("console.log(password"), "no password logging");

// 8. Security section + custom-anisette warning in Settings.
const settings = read("src/pages/Settings.tsx");
ok(settings.includes("security_heading"), "security section");
ok(settings.includes("custom_anisette_title"), "custom anisette warning");
ok(read("src/locales/en.json").includes("security_local"), "en security keys");
ok(read("src/locales/de.json").includes("security_local"), "de security keys");

// 9. UI polish: about dialog, close label, net list, 2FA/login states.
ok(read("src/locales/en.json").includes('"tagline"'), "en about keys");
ok(read("src/locales/de.json").includes('"tagline"'), "de about keys");
ok(settings.includes("net_apple"), "net list i18n");
ok(read("src/AppleID.tsx").includes("loginBusy"), "login busy state");
ok(read("src/AppleID.tsx").includes("two_factor_hint"), "2fa hint");
ok(read("src/components/Modal.tsx").includes("aria-label"), "modal close label");
ok(read("src/Device.css").includes(".spinner"), "pairing spinner styled");
ok(read("src/App.css").includes("prefers-reduced-motion"), "reduced motion");
ok(read("src/App.css").includes("focus-visible"), "focus rings");

// 7. Docs.
ok(read("PRIVACY.md").includes("DenizSigner"), "PRIVACY.md");
ok(read("NOTICE.md").includes("iloader"), "NOTICE credits upstream");

console.log(`frontend tests: ${n}/${n} passed`);
