#!/usr/bin/env node
// Renders the FormaCAD Homebrew cask for a stable version from the release SHA256SUMS.
// Usage: node scripts/homebrew-cask.mjs <version> <SHA256SUMS path> [output path]
// The committed packaging/homebrew/Casks/formacad.rb is this output; the release workflow pushes it to the tap.
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export function dmgName(version, arch) {
  return arch === "arm" ? `darwin-aarch64-Forma_${version}_aarch64.dmg` : `darwin-x86_64-Forma_${version}_x64.dmg`;
}

export function hashesFromSums(text, version) {
  const sums = new Map(text.split(/\r?\n/).filter(Boolean).map((line) => {
    const [hash, name] = line.trim().split(/\s+\*?/);
    return [name, hash.toLowerCase()];
  }));
  const pick = (arch) => {
    const hash = sums.get(dmgName(version, arch));
    if (!/^[0-9a-f]{64}$/.test(hash ?? "")) throw new Error(`SHA256SUMS has no hash for ${dmgName(version, arch)}`);
    return hash;
  };
  return { arm: pick("arm"), intel: pick("intel") };
}

export function renderCask(version, { arm, intel }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`Only stable x.y.z versions go to Homebrew, got ${version}`);
  return `cask "formacad" do
  arch arm: "aarch64", intel: "x64"
  arch_dir = on_arch_conditional arm: "aarch64", intel: "x86_64"

  version "${version}"
  sha256 arm:   "${arm}",
         intel: "${intel}"

  url "https://github.com/lovlyDev/FormaCAD/releases/download/v#{version}/darwin-#{arch_dir}-Forma_#{version}_#{arch}.dmg",
      verified: "github.com/lovlyDev/FormaCAD/"
  name "FormaCAD"
  desc "Local CAD workspace with AI-assisted parametric modeling"
  homepage "https://formacad.ru/"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: ">= :big_sur"

  app "Forma.app"

  # The app is not notarized yet; drop the download quarantine so Gatekeeper does not block the first launch.
  postflight do
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/Forma.app"], sudo: false
  end

  zap trash: [
    "~/Library/Application Support/studio.forma.cad",
    "~/Library/Caches/studio.forma.cad",
    "~/Library/Preferences/studio.forma.cad.plist",
    "~/Library/WebKit/studio.forma.cad",
  ]
end
`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [version, sumsPath, out] = process.argv.slice(2);
  if (!version || !sumsPath) {
    console.error("Usage: node scripts/homebrew-cask.mjs <version> <SHA256SUMS> [output]");
    process.exit(2);
  }
  const cask = renderCask(version, hashesFromSums(readFileSync(sumsPath, "utf8"), version));
  if (out) writeFileSync(out, cask); else process.stdout.write(cask);
}
