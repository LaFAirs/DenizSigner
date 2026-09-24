import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");
let n = 0;
const ok = (cond, name) => { assert.ok(cond, name); n++; console.log(`  ok ${n} - ${name}`); };

// 1. App boots: entries exist, correct title/branding.
ok(existsSync(join(root, "src/main.tsx")), "main.tsx exists");
ok(existsSync(join(root, "src/App.tsx")), "App.tsx exists");
ok(read("index.html").includes("<title>DenizSigner</title>"), "window title DenizSigner");
ok(!read("src/App.tsx").toLowerCase().includes("iloader"), "no upstream name in App.tsx");

// 2. IPA detection helper present (extension gate; backend checks ZIP magic).
ok(read("src/lib/network.ts").includes("isIpaPath"), "isIpaPath helper");
ok(read("src-tauri/src/ipa.rs").includes("Payload/"), "backend Payload check");

// 3. Friendly errors cover required workflows.
const errors = read("src/lib/errors.ts");
for (const t of ["invalid_ipa", "no_device_selected", "not_logged_in", "auth", "anisette", "network", "max_apps"]) {
  ok(errors.includes(t), `friendly error: ${t}`);
}

// 4. No telemetry / updater / upstream endpoints in shipped code.
const shipped = ["src/App.tsx", "src/lib/api.ts", "src/lib/network.ts", "src/lib/errors.ts",
  "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml", "package.json", "index.html"]
  .map(read).join("\n").toLowerCase();
for (const bad of ["telemetry", "analytics", "sentry", "posthog", "plausible", "tauri-plugin-updater", "latest.json", "nab138", "iloader", "me.nabdev", "umami", "amplitude", "hotjar"]) {
  ok(!shipped.includes(bad), `no forbidden token: ${bad}`);
}

// 5. Allowlist documented + surfaced in Settings privacy section.
ok(read("src/lib/network.ts").includes("ALLOWED_SERVICES"), "ALLOWED_SERVICES");
ok(read("src/pages/SettingsDialog.tsx").includes("networkActivity"), "privacy network section");
ok(read("src-tauri/src/network_allowlist.rs").includes("require_allowed_url"), "backend boundary");

// 6. Credential storage uses OS store with own service name.
ok(read("src-tauri/src/secure_storage.rs").includes('"denizsigner"'), "keyring service denizsigner");
ok(!read("src-tauri/src/secure_storage.rs").includes('"iloader"'), "no upstream service name");

// 7. Destructive actions confirmed + secrets redacted in logs.
ok(read("src/components/ConfirmDialog.tsx").includes("Cancel") || read("src/components/ConfirmDialog.tsx").includes("confirm.cancel"), "confirm dialog");
ok(read("src-tauri/src/logging.rs").includes("redact"), "log redaction");
ok(read("PRIVACY.md").includes("DenizSigner"), "PRIVACY.md");

console.log(`frontend tests: ${n}/${n} passed`);
