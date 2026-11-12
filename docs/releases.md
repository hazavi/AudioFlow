# Signed Windows releases

Releases are published from [hazavi/AudioFlow](https://github.com/hazavi/AudioFlow). The application is configured to consume `https://github.com/hazavi/AudioFlow/releases/latest/download/latest.json`. That endpoint becomes available when the release workflow publishes a signed release and its manifest.

## Configure once

1. Preserve the committed `src-tauri/updater.pub`; installed versions trust this key.
2. If configuring a fresh checkout, run `npm run release:configure -- hazavi/AudioFlow`. This saves the repository's HTTPS manifest endpoint and public key in `src-tauri/tauri.conf.json`. The release workflow supplies the same repository automatically through `GITHUB_REPOSITORY`.
3. Commit the changed configuration before distributing the first update-enabled installer. Builds shipped without an endpoint cannot discover a repository created later; they need this first configured installer once.
4. Add the **content** of `%USERPROFILE%\.audioflow\release.key` to the GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY`. Keep a separate secure backup. Never print it in an issue, commit it, or put it into a Vite variable.
5. The generated key has an empty password. Set `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to the matching password if you replace it with a password-protected key before the first release. `AUDIOFLOW_UPDATER_PUBLIC_KEY` is an optional repository variable to supply the same public key; the committed public key is the default.

The local key was generated through Tauri's signer outside the source repository. If starting with a fresh clone without that private key, recover the original key from its secure backup. Do not create a new key for users who already trust the old one. The configure script only creates a new key when no public key or previous key exists, and never generates keys in CI.

## Publish

Update the stable version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`. Push a matching tag such as `v0.2.0`.

`.github/workflows/release.yml` installs Node and Rust, validates the tag and versions, runs tests, builds the Windows x86_64 NSIS installer, generates updater artifacts, and uses the official `tauri-apps/tauri-action` to sign and upload a draft release. The action creates `latest.json`. The workflow downloads the uploaded manifest, installer, and `.sig`, checks their URLs and Windows platform keys, cryptographically verifies the installer with the public key, checks the signature-bound version, and only then publishes the draft as the latest stable release. A failed step leaves it unpublished. The workflow uses `contents: write`; it does not need a personal access token.

For a local signed build:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Join-Path $env:USERPROFILE '.audioflow\release.key'
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
npm run tauri -- build --bundles nsis
```

The resulting installer and its Tauri `.sig` are under `src-tauri/target/release/bundle/nsis`. `createUpdaterArtifacts` is enabled. HTTPS and Tauri signature verification are mandatory; version signatures are required. No arbitrary executable download-and-launch fallback exists.

## Validate an upgrade

Install a configured older version on a test Windows account, save assignments and rules, and publish a higher stable release with the same key. Check manually, download, defer once, then install with consent. Verify settings survive, the running process exits cleanly, the new version launches, and tray scheduling continues. Test an offline check and a tampered artifact, which must fail without reporting success or destroying configuration.

Tauri updater signatures establish update-package authenticity. They are separate from Windows Authenticode certificates. This project has no Authenticode certificate, so SmartScreen may warn on downloaded installers. Do not describe the installer as Windows-trusted or bypass a warning automatically. Add Authenticode signing to the build when an appropriate certificate is available.

Official references: [Tauri updater](https://v2.tauri.app/plugin/updater/), [Tauri GitHub distribution](https://v2.tauri.app/distribute/pipelines/github/), and [tauri-action](https://github.com/tauri-apps/tauri-action).
