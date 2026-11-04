cask "formacad" do
  arch arm: "aarch64", intel: "x64"
  arch_dir = on_arch_conditional arm: "aarch64", intel: "x86_64"

  version "1.6.1"
  sha256 arm:   "1afa13bd02453a7783025ad3b97ccf8d53fc3a4c12104e204876f86f8219b807",
         intel: "f7b68b6a2d79515c1501459fdcffc3323c05f03e5087aa4bfe8024ec1db3c63d"

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
