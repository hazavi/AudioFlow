import { afterEach, describe, expect, it } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const script = fileURLToPath(
  new URL("./normalize-update-manifest.mjs", import.meta.url),
);
const prefix = path.join(os.tmpdir(), "audioflow-manifest-test-");
const temporary: string[] = [];
afterEach(() => {
  for (const directory of temporary.splice(0)) {
    if (!path.resolve(directory).startsWith(prefix))
      throw new Error("Unexpected test directory.");
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
function fixture() {
  const directory = fs.mkdtempSync(prefix);
  temporary.push(directory);
  const assets = path.join(directory, "assets");
  fs.mkdirSync(assets);
  fs.mkdirSync(path.join(directory, "src-tauri"));
  const name = "AudioFlow_0.1.0_x64-setup.exe";
  const url =
    "https://api.github.com/repos/hazavi/AudioFlow/releases/assets/123";
  const canonical = `https://github.com/hazavi/AudioFlow/releases/download/v0.1.0/${name}`;
  const release = {
    tag_name: "v0.1.0",
    assets: [{ id: 123, name, size: 4, url, browser_download_url: canonical }],
  };
  const manifest = {
    version: "0.1.0",
    platforms: {
      "windows-x86_64-nsis": { url, signature: "metadata-test-only" },
    },
  };
  const write = () => {
    fs.writeFileSync(
      path.join(assets, "latest.json"),
      JSON.stringify(manifest),
    );
    fs.writeFileSync(
      path.join(assets, "release.json"),
      JSON.stringify(release),
    );
  };
  fs.writeFileSync(path.join(assets, name), "test");
  fs.writeFileSync(
    path.join(directory, "src-tauri/tauri.conf.json"),
    JSON.stringify({
      version: "0.1.0",
      plugins: {
        updater: {
          endpoints: [
            "https://github.com/hazavi/AudioFlow/releases/latest/download/latest.json",
          ],
        },
      },
    }),
  );
  write();
  const run = () =>
    spawnSync(process.execPath, [script, assets], {
      cwd: directory,
      encoding: "utf8",
    });
  return { assets, manifest, release, canonical, write, run };
}
describe("release manifest URLs", () => {
  it("converts an API asset URL to the versioned public installer URL", () => {
    const f = fixture();
    expect(f.run().status).toBe(0);
    expect(
      JSON.parse(fs.readFileSync(path.join(f.assets, "latest.json"), "utf8"))
        .platforms["windows-x86_64-nsis"].url,
    ).toBe(f.canonical);
  });
  it("preserves a matching canonical URL", () => {
    const f = fixture();
    f.manifest.platforms["windows-x86_64-nsis"].url = f.canonical;
    f.write();
    expect(f.run().status).toBe(0);
  });
  it("rejects an unlisted download URL", () => {
    const f = fixture();
    f.manifest.platforms["windows-x86_64-nsis"].url =
      "https://untrusted.invalid/installer.exe";
    f.write();
    expect(f.run().status).not.toBe(0);
  });
  it("rejects metadata from another release", () => {
    const f = fixture();
    f.release.tag_name = "v0.0.9";
    f.write();
    expect(f.run().status).not.toBe(0);
  });
});
