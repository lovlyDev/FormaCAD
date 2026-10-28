// Administration of the "FormaCAD Plugins" GitHub App (catalog publishing) with an App JWT.
// The private key never enters the repository: pass its path through the environment.
//   GITHUB_APP_ID=<id> GITHUB_APP_PRIVATE_KEY_PATH=<path to .pem outside Git> node scripts/maintainers/github-app-admin.mjs <command>
// Commands:
//   installations            list installations (account, type, repository selection)
//   deliveries [n]           last webhook deliveries (default 10)
//   redeliver <delivery id>  ask GitHub to resend a webhook delivery
//   set-secret <file>        set the webhook secret from a file (the file stays outside Git)
import fs from "node:fs";
import crypto from "node:crypto";

const appId = process.env.GITHUB_APP_ID;
const keyPath = process.env.GITHUB_APP_PRIVATE_KEY_PATH;
if (!appId || !keyPath) throw new Error("Set GITHUB_APP_ID and GITHUB_APP_PRIVATE_KEY_PATH");

const b64 = (value) => Buffer.from(JSON.stringify(value)).toString("base64url");
const now = Math.floor(Date.now() / 1000);
const unsigned = `${b64({ alg: "RS256", typ: "JWT" })}.${b64({ iat: now - 60, exp: now + 300, iss: appId })}`;
const jwt = `${unsigned}.${crypto.sign("RSA-SHA256", Buffer.from(unsigned), fs.readFileSync(keyPath)).toString("base64url")}`;

const api = (route, options = {}) =>
  fetch(`https://api.github.com${route}`, {
    ...options,
    headers: { authorization: `Bearer ${jwt}`, accept: "application/vnd.github+json", ...(options.headers || {}) },
  });

const [command, arg] = process.argv.slice(2);
if (command === "installations") {
  for (const i of await (await api("/app/installations")).json())
    console.log(i.id, i.account.login, i.account.type, i.repository_selection, i.created_at);
} else if (command === "deliveries") {
  for (const d of await (await api(`/app/hook/deliveries?per_page=${Number(arg) || 10}`)).json())
    console.log(d.id, d.delivered_at, d.event, d.action, d.status_code, d.redelivery);
} else if (command === "redeliver" && arg) {
  console.log("redeliver", (await api(`/app/hook/deliveries/${arg}/attempts`, { method: "POST" })).status);
} else if (command === "set-secret" && arg) {
  const secret = fs.readFileSync(arg, "utf8").trim();
  const response = await api("/app/hook/config", {
    method: "PATCH",
    body: JSON.stringify({ secret }),
    headers: { "content-type": "application/json" },
  });
  console.log("set-secret", response.status);
} else {
  console.log("Usage: github-app-admin.mjs installations | deliveries [n] | redeliver <id> | set-secret <file>");
  process.exit(2);
}
