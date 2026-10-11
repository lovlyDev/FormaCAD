import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  // GitHub's software WebGL renderer serializes screenshots across pages.
  workers: process.env.CI ? 1 : 2,
  timeout: 90000,
  expect: { timeout: 15000 },
  use: {
    baseURL: "http://127.0.0.1:1420",
    viewport: { width: 1440, height: 960 },
    launchOptions: {
      args: [
        // SwiftShader only backs WebGL; software 2D compositing avoids the
        // GPU-replacement screenshot stall while retaining real WebGL2 tests.
        "--use-gl=angle",
        "--use-angle=swiftshader-webgl",
        "--enable-unsafe-swiftshader",
        "--enable-webgl",
        "--ignore-gpu-blocklist",
      ],
    },
  },
  webServer: {
    command: "node ../../node_modules/vite/bin/vite.js --host 127.0.0.1",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
  },
  reporter: "list",
});
