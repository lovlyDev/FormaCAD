import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";

const directory = "apps/desktop/src-tauri/migrations";
const originalChecksum = "7de65b5e9ffddba18ebf13a8d4313e4eb5613e204fb1326c1afe78c8a769f148942e68d0aa407eb44c586602402cc49d";

for (const file of readdirSync(directory).filter((name) => name.endsWith(".sql"))) {
  const content = readFileSync(`${directory}/${file}`);
  if (content.includes("\r")) {
    throw new Error(`${file} must use LF line endings on every platform`);
  }
  if (file === "0001_initial.sql") {
    const checksum = createHash("sha384").update(content).digest("hex");
    if (checksum !== originalChecksum) {
      throw new Error("Migration 1 was already released and must not be changed; add a new migration instead");
    }
  }
}

console.log("SQL migrations use stable LF line endings and the original migration is unchanged");
