import fs from "node:fs";
const paths = ["package.json", "apps/desktop/package.json", "apps/desktop/src-tauri/tauri.conf.json"];
const desired = process.argv[2];
const version = desired || JSON.parse(fs.readFileSync(paths[0], "utf8")).version;
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error("Use a stable x.y.z version");
for (const path of paths) {
  const data = JSON.parse(fs.readFileSync(path, "utf8"));
  if (desired) { data.version = version; fs.writeFileSync(path, JSON.stringify(data, null, 2) + "\n"); }
  else if (data.version !== version) throw new Error(`Version mismatch: ${path}`);
}
const lock = JSON.parse(fs.readFileSync("package-lock.json", "utf8"));
if (desired) {
  lock.version = lock.packages[""].version = lock.packages["apps/desktop"].version = version;
  fs.writeFileSync("package-lock.json", JSON.stringify(lock, null, 2) + "\n");
} else if ([lock.version,lock.packages[""].version,lock.packages["apps/desktop"].version].some(v => v !== version)) throw new Error("npm lockfile version mismatch");
for (const path of ["apps/desktop/src-tauri/Cargo.toml", "apps/desktop/src-tauri/Cargo.lock"]) {
  let text = fs.readFileSync(path, "utf8");
  const pattern = /(name = "forma-cad"\r?\nversion = ")[^"]+(")/;
  const current = /name = "forma-cad"\r?\nversion = "([^"]+)"/.exec(text)?.[1];
  if (desired) {text = text.replace(pattern, `$1${version}$2`);fs.writeFileSync(path,text);}
  else if (current !== version) throw new Error(`Version mismatch: ${path}`);
}
if (process.env.GITHUB_REF_TYPE === "tag" && process.env.GITHUB_REF_NAME !== `v${version}`) throw new Error("Release tag must match package version");
console.log(`Forma ${version}: versions synchronized`);
