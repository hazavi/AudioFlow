# Security

Report vulnerabilities through [GitHub private vulnerability reporting](https://github.com/hazavi/AudioFlow/security/advisories/new). Include the AudioFlow and Windows versions, reproduction steps, and the affected feature. Redact credentials and personal data from screenshots and logs.

AudioFlow runs as the current Windows user. Its main webview accepts bundled content and blocks external navigation and popups. Native commands are granted explicitly to that local window. The content security policy restricts scripts, connections, frames, and forms. The frontend freezes `Object.prototype` before loading application code.

GitHub and release links open through native commands that accept no URL argument. Update packages require HTTPS, the committed verification key, and a matching signed version. Installation requires confirmation in the app.

The signing private key and its optional password belong in encrypted GitHub Actions secrets. The release workflow supplies them only to the signing step. Public verification keys ship with the app. Keep credentials outside the source tree and out of `VITE_` variables, which are included in frontend builds. Configuration backups and exported activity can contain application names and local paths.

CI scans Git history for secrets and audits npm and Rust dependencies on pushes, pull requests, and a weekly schedule. Workflow actions use immutable commit references. Secret scanning and push protection are enabled on GitHub.

The November 13, 2026 audit found zero npm advisories and zero Rust vulnerability findings. The Rust lockfile has informational advisories for `proc-macro-error`, `zbus_names`, `zvariant`, `zvariant_derive`, and `glib`. The Windows dependency graph excludes these packages. The whole-lockfile audit continues to report them so their status remains visible.

Tauri updater signatures authenticate update packages. Windows Authenticode signing requires a separate certificate, which this project currently lacks.
