import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { spawnSync } from "node:child_process";

const repository = process.argv[2] || process.env.GITHUB_REPOSITORY;
if (!repository || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
  throw new Error(
    "Pass the actual GitHub owner/repository after creating it: npm run release:configure -- owner/repository",
  );
}
const configPath = "src-tauri/tauri.conf.json";
const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
let pubkey =
  process.env.AUDIOFLOW_UPDATER_PUBLIC_KEY ||
  config.plugins.updater?.pubkey ||
  (fs.existsSync("src-tauri/updater.pub")
    ? fs.readFileSync("src-tauri/updater.pub", "utf8").trim()
    : null);
if (!pubkey) {
  if (process.env.CI)
    throw new Error(
      "Set the AUDIOFLOW_UPDATER_PUBLIC_KEY repository variable. CI must never generate a replacement key.",
    );
  const key = path.join(os.homedir(), ".audioflow", "release.key");
  if (!fs.existsSync(`${key}.pub`)) {
    if (fs.existsSync(key))
      throw new Error(
        `The private key already exists at ${key}; recover its public key before continuing.`,
      );
    fs.mkdirSync(path.dirname(key), { recursive: true });
    const cli = path.resolve("node_modules/@tauri-apps/cli/tauri.js");
    const result = spawnSync(
      process.execPath,
      [cli, "signer", "generate", "--ci", "-w", key],
      { encoding: "utf8", env: process.env },
    );
    // Never echo CLI output: it can contain signing material.
    if (result.status !== 0)
      throw new Error(
        "Key generation failed. Generate a key with the Tauri signer outside the repository and supply AUDIOFLOW_UPDATER_PUBLIC_KEY.",
      );
    console.log(
      `Signing key created outside the repository: ${key}. Back it up securely and add it to GitHub Actions secrets.`,
    );
  }
  pubkey = fs.readFileSync(`${key}.pub`, "utf8").trim();
}
const decoded = Buffer.from(pubkey, "base64").toString("utf8");
if (
  !decoded.includes("untrusted comment:") ||
  !decoded.split(/\r?\n/).some((line) => /^RW[A-Za-z0-9+/=]+$/.test(line))
)
  throw new Error("The updater public key is not a Tauri/minisign public key.");
const endpoint = `https://github.com/${repository}/releases/latest/download/latest.json`;
config.plugins.updater = {
  pubkey,
  endpoints: [endpoint],
  requireSignedVersion: true,
  windows: { installMode: "passive" },
};
fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
console.log(`Configured signed updates from ${endpoint}`);
