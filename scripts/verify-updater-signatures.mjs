// Verifies every updater signature (`<file>.sig`) in a directory against the public key
// from tauri.conf.json, then checks SHA256SUMS when present.
// Usage: node scripts/verify-updater-signatures.mjs [dir=.release-artifacts] [tauri.conf.json]
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const dir = process.argv[2] || ".release-artifacts";
const confPath = process.argv[3] || "apps/desktop/src-tauri/tauri.conf.json";

// minisign: base64 of a two-line text; line 2 is base64 of algorithm(2) + key id(8) + payload.
const minisignBlob = (text) => Buffer.from(Buffer.from(text.trim(), "base64").toString("utf8").split("\n")[1], "base64");
const publicKey = minisignBlob(JSON.parse(fs.readFileSync(confPath, "utf8")).plugins.updater.pubkey);
const ed25519 = crypto.createPublicKey({
  key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), publicKey.subarray(10)]),
  format: "der",
  type: "spki",
});

function verify(file, signatureText) {
  const signature = minisignBlob(signatureText);
  if (!signature.subarray(2, 10).equals(publicKey.subarray(2, 10))) return "key id mismatch";
  const data = fs.readFileSync(file);
  // "ED" signs the BLAKE2b-512 hash (prehashed), "Ed" signs the file bytes.
  const message = signature.subarray(0, 2).toString() === "ED" ? crypto.createHash("blake2b512").update(data).digest() : data;
  return crypto.verify(null, message, ed25519, signature.subarray(10)) ? "" : "bad signature";
}

const files = fs.readdirSync(dir, { recursive: true }).map(String).filter((f) => fs.statSync(path.join(dir, f)).isFile());
const signatures = files.filter((f) => f.endsWith(".sig"));
if (!signatures.length) throw new Error(`No .sig files in ${dir}`);
let failed = 0;
for (const sig of signatures) {
  const target = path.join(dir, sig.slice(0, -4));
  const problem = fs.existsSync(target) ? verify(target, fs.readFileSync(path.join(dir, sig), "utf8")) : "signed file missing";
  console.log(`${problem ? "FAIL" : "OK  "} ${sig.slice(0, -4)}${problem ? ` (${problem})` : ""}`);
  if (problem) failed++;
}

const sumsPath = path.join(dir, "SHA256SUMS");
if (fs.existsSync(sumsPath)) {
  for (const line of fs.readFileSync(sumsPath, "utf8").split("\n").filter(Boolean)) {
    const [hash, name] = line.split(/\s+/);
    const file = path.join(dir, name);
    const actual = fs.existsSync(file) ? crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex") : "";
    if (actual !== hash) { console.log(`FAIL SHA256 ${name}`); failed++; }
  }
  console.log("SHA256SUMS checked");
}
if (failed) { console.error(`${failed} check(s) failed`); process.exit(1); }
console.log("All updater signatures are valid");
