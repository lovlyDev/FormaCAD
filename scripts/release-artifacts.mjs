import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
const version = JSON.parse(fs.readFileSync("package.json", "utf8")).version;
const platforms = ["windows-x86_64", "linux-x86_64", "darwin-aarch64", "darwin-x86_64"];
const [mode, platform, target] = process.argv.slice(2);
const root = ".release-artifacts";
if (mode === "collect") {
  if (!platforms.includes(platform) || !/^[\w-]+$/.test(target)) throw new Error("Invalid target");
  const bundle = `apps/desktop/src-tauri/target/${target}/release/bundle`;
  const output = path.join(root, platform);
  fs.mkdirSync(output, { recursive: true });
  const files = fs.readdirSync(bundle, {recursive:true}).filter(f => /\.(exe|dmg|deb|rpm|AppImage|app\.tar\.gz)(\.sig)?$/.test(f));
  if (!files.length) throw new Error("No installers produced");
  let updater;
  for (const file of files) {
    const name = `${platform}-${path.basename(file)}`;
    fs.copyFileSync(path.join(bundle,file),path.join(output,name));
    if ((platform.startsWith("windows") && file.endsWith(".exe")) || (platform.startsWith("linux") && file.endsWith(".AppImage")) || (platform.startsWith("darwin") && file.endsWith(".app.tar.gz"))) {
      const signature = fs.readFileSync(path.join(bundle,`${file}.sig`),"utf8").trim();
      if (!signature) throw new Error("Missing updater signature");
      updater = {name,signature};
    }
  }
  if (!updater) throw new Error(`Missing updater for ${platform}`);
  fs.writeFileSync(path.join(output,`manifest-${platform}.json`),JSON.stringify({version,platform,...updater}));
} else if (mode === "merge") {
  const repository = process.env.GITHUB_REPOSITORY || "lovlyDev/FormaCAD";
  const manifest = {version,notes:fs.readFileSync(`docs/releases/${version}.md`,"utf8"),pub_date:new Date().toISOString(),platforms:{}};
  for (const platform of platforms) {
    const file=path.join(root,`manifest-${platform}.json`);
    const part=JSON.parse(fs.readFileSync(file,"utf8"));
    if(part.version !== version || part.platform !== platform || path.basename(part.name)!==part.name || !part.signature || !fs.existsSync(path.join(root,part.name))) throw new Error(`Invalid ${platform} artifact`);
    manifest.platforms[platform]={signature:part.signature,url:`https://github.com/${repository}/releases/download/v${version}/${encodeURIComponent(part.name)}`};
    fs.unlinkSync(file);
  }
  fs.writeFileSync(path.join(root,"latest.json"),JSON.stringify(manifest,null,2)+"\n");
  const hashes=fs.readdirSync(root).filter(f=>fs.statSync(path.join(root,f)).isFile() && f!=="SHA256SUMS").sort().map(f=>`${crypto.createHash("sha256").update(fs.readFileSync(path.join(root,f))).digest("hex")}  ${f}`);
  fs.writeFileSync(path.join(root,"SHA256SUMS"),hashes.join("\n")+"\n");
} else throw new Error("Use collect PLATFORM TARGET or merge");
