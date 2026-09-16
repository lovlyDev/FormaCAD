import fs from "node:fs";

const configPath = "apps/desktop/src-tauri/tauri.conf.json";
const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
const repository = process.env.GITHUB_REPOSITORY || "lovlyDev/FormaCAD";
if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) throw new Error("Invalid GitHub repository");
const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim() || config.plugins?.updater?.pubkey;
if (!pubkey) throw new Error("Set TAURI_UPDATER_PUBLIC_KEY to the updater public key.");
config.plugins = {...config.plugins, updater: {pubkey, endpoints: [`https://github.com/${repository}/releases/latest/download/latest.json`], windows: {installMode: "passive"}}};
config.bundle.createUpdaterArtifacts = true;
fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + "\n");
console.log(`Configured signed updates for ${repository}`);
