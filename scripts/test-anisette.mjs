import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
const source = readFileSync(new URL("../src/anisette.ts", import.meta.url), "utf8");
const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } }).outputText;
const { migrateAnisetteServer: migrate, DEFAULT_ANISETTE_SERVER: current } =
  await import("data:text/javascript;base64," + Buffer.from(js).toString("base64"));
for (const value of [undefined, null, "", "ani.sidestore.io", "https://ANI.SIDESTORE.IO/", "ani.stikstore.app", "ani.sidestore.app", "ani.sidestore.zip", "ani.846969.xyz", "ani.neoarz.xyz", "ani.xu30.top", "anisette.wedotstud.io"]) {
  assert.equal(migrate(value), current, `migrate legacy/default ${value}`);
}
for (const custom of [current, "new.ani.sidestore.io", "ani3.stikstore.app", "my-ani.example.org", "https://custom.example.org:8443", "ani.sidestore.io.evil.example", "https://ani.sidestore.io:8443", "https://ani.sidestore.io/path"]) {
  assert.equal(migrate(custom), custom, `preserve custom ${custom}`);
}
console.log("Anisette migration: 20 cases passed");
