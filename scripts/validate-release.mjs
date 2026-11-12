import fs from "node:fs";
import path from "node:path";
const config = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
const cargo = fs
  .readFileSync("src-tauri/Cargo.toml", "utf8")
  .match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const tag = process.env.GITHUB_REF_NAME;
if (
  config.version !== pkg.version ||
  config.version !== cargo ||
  (tag && tag !== `v${config.version}`)
)
  throw new Error(
    "Tag, package.json, Cargo.toml, and tauri.conf.json must have the same stable version.",
  );
if (!/^\d+\.\d+\.\d+$/.test(config.version))
  throw new Error(
    "Only stable semantic versions may use this release workflow.",
  );
const updater = config.plugins.updater;
if (
  !updater?.pubkey ||
  !updater.requireSignedVersion ||
  !updater.endpoints?.length
)
  throw new Error(
    "Configure a public signing key and GitHub repository before releasing.",
  );
const expected = process.env.GITHUB_REPOSITORY
  ? `https://github.com/${process.env.GITHUB_REPOSITORY}/releases/latest/download/latest.json`
  : updater.endpoints[0];
if (
  updater.endpoints[0] !== expected ||
  !expected.startsWith("https://github.com/")
)
  throw new Error(
    "The update endpoint must point to the current GitHub repository over HTTPS.",
  );
if (!config.bundle.createUpdaterArtifacts)
  throw new Error("Updater artifacts must be enabled.");
const dir = process.argv[2];
if (dir) {
  const manifest = JSON.parse(
    fs.readFileSync(path.join(dir, "latest.json"), "utf8"),
  );
  if (manifest.version.replace(/^v/, "") !== config.version)
    throw new Error("Update manifest version is incorrect.");
  const platform =
    manifest.platforms?.["windows-x86_64-nsis"] ||
    manifest.platforms?.["windows-x86_64"];
  if (!platform?.signature || !platform.url)
    throw new Error("Missing Windows x86_64 signed update entry.");
  const url = new URL(platform.url);
  const repo = expected.slice(
    0,
    -"/releases/latest/download/latest.json".length,
  );
  if (
    !platform.url.startsWith(`${repo}/releases/download/v${config.version}/`) ||
    url.protocol !== "https:"
  )
    throw new Error(
      "Artifact URL is not a versioned release in this repository.",
    );
  const name = decodeURIComponent(path.posix.basename(url.pathname));
  const artifact = path.join(dir, name);
  if (
    !name.endsWith(".exe") ||
    !fs.existsSync(artifact) ||
    fs.statSync(artifact).size < 100000
  )
    throw new Error("The Windows NSIS updater artifact is missing or empty.");
  const signature = fs.readFileSync(`${artifact}.sig`, "utf8").trim();
  if (signature !== platform.signature.trim())
    throw new Error(
      "Manifest signature differs from the generated signature file.",
    );
  console.log(
    `Validated manifest, installer, and signature for v${config.version}.`,
  );
}
console.log(`Release configuration is valid: v${config.version}`);
