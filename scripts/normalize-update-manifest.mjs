import fs from "node:fs";
import path from "node:path";

const directory = process.argv[2];
if (!directory)
  throw new Error("Pass the downloaded release-assets directory.");
const readJson = (file) =>
  JSON.parse(fs.readFileSync(file, "utf8").replace(/^\uFEFF/, ""));
const config = readJson("src-tauri/tauri.conf.json");
const manifestPath = path.join(directory, "latest.json");
const manifest = readJson(manifestPath);
const release = readJson(path.join(directory, "release.json"));
const endpoint = config.plugins?.updater?.endpoints?.[0];
const match =
  /^https:\/\/github\.com\/([^/]+\/[^/]+)\/releases\/latest\/download\/latest\.json$/.exec(
    endpoint || "",
  );
if (!match)
  throw new Error("The configured endpoint is not a GitHub release manifest.");
const repository = match[1];
const tag = `v${config.version}`;
if (
  release.tag_name !== tag ||
  manifest.version.replace(/^v/, "") !== config.version
) {
  throw new Error(
    "Release metadata and manifest must match the application version.",
  );
}
if (
  !Array.isArray(release.assets) ||
  !manifest.platforms ||
  !Object.keys(manifest.platforms).length
) {
  throw new Error("Release assets and update platforms are required.");
}
for (const [platform, update] of Object.entries(manifest.platforms)) {
  if (!["windows-x86_64", "windows-x86_64-nsis"].includes(platform)) {
    throw new Error(`Unsupported release platform: ${platform}`);
  }
  const asset = release.assets.find((candidate) => {
    const canonical = `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(candidate.name)}`;
    return [candidate.url, candidate.browser_download_url, canonical].includes(
      update.url,
    );
  });
  if (!asset || !/^[A-Za-z0-9_.-]+\.exe$/.test(asset.name)) {
    throw new Error(
      "The update URL does not identify a known Windows installer asset.",
    );
  }
  if (
    asset.url !==
    `https://api.github.com/repos/${repository}/releases/assets/${asset.id}`
  ) {
    throw new Error("The installer asset belongs to another repository.");
  }
  if (fs.statSync(path.join(directory, asset.name)).size !== asset.size) {
    throw new Error(
      "The downloaded installer size does not match the release metadata.",
    );
  }
  update.url = `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(asset.name)}`;
}
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Normalized verified release-asset URLs for ${tag}.`);
