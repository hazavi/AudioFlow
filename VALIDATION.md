# Validation record

Tested on Windows on 12 November 2026. Automated checks and physical audio-session observations are recorded separately.

## v0.1.5 verification

Verified on Windows on 13 November 2026.

- Passed 15 Rust tests, 15 frontend tests, and the production frontend build.
- New regression tests run the actual worker reconciliation loop against a mock Windows provider. They check 100 focus and enumeration changes without extra route writes, temporary missing and failed session enumeration, audio-child restarts, owner PID reuse, and moving the tracked window between monitors.
- Verified that playback gaps preserve ownership with either restore-on-exit setting, while explicit restoration still restores every original role preference.

These regression checks use simulated Windows state and do not play or reroute the user's applications. The reported browser playback issue still needs confirmation after installing this version.

## v0.1.4 verification

- Passed the production frontend build and 15 frontend tests.
- Verified the source ICO and installed executable both contain the waveform icon.
- Tested an isolated NSIS install and update. An existing shortcut with a default executable icon was updated to the versioned waveform ICO, while its target and custom arguments were preserved. The Start menu shortcut was updated too.
- Verified uninstall removed the test shortcut and versioned icon after its background process completed.

## v0.1.3 verification

Verified on Windows on 13 November 2026.

- Passed 15 frontend tests, 12 Rust tests, and the production frontend build. The new tests cover navigation restrictions, repository URL validation, the confirmed update sequence, and download/signature failure handling.
- Built an isolated NSIS package with a separate application identity. Verified same-version update-mode replacement while the app was running, a delayed graceful exit, automatic restart, and preservation of saved settings and assignments. Uninstall also completed while that app was running.
- Reproduced the installer failure when Restart Manager listed McAfee Framework Host as a file user. The corrected check matched the installed executable's process path, and both update and uninstall passed while McAfee remained running.
- Confirmed the bundled UI loaded under the new content policy and native permissions, including the GitHub title-bar control.
- Git history and built frontend secret scans passed. The npm audit reported zero vulnerabilities. Rust audit reported zero vulnerability findings and five informational advisories in packages excluded from the Windows dependency graph, as listed in [security details](SECURITY.md).

These installer checks used an isolated debug package. An installed production older-to-newer in-app upgrade remains a separate check.

## Automated checks

- Frontend production build: passed with TypeScript checking and Vite bundling.
- Frontend tests: 9 passed. These cover update-check serialization, disabled checks, offline errors, actual byte accounting, installation consent boundaries, failed-install recovery, bidirectional theme switching, live system-theme changes, and preference persistence.
- Rust unit tests: 8 passed. These cover negative and vertical monitor coordinates, largest intersection and hysteresis, focused and previous windows, manual preferences and explicit rules, disconnected endpoints and fallback, stable-target delay, exact-role restoration, external manual changes, PID reuse, and configuration persistence and validation.
- The all-target Rust check passed, including the release-signature verifier and hardware helper.
- The two opt-in native hardware tests are ignored by ordinary CI: one is a helper process and the other requires physical hardware. The hardware test was run separately, as described below.

## Hardware observations

The read-only Windows probe found a primary 1920 × 1200 display at (0, 0), a second 1920 × 1080 display at (0, -1080), two playback endpoints, and five audio sessions. It successfully queried the internal Windows per-process policy interface.

The isolated hardware test copied its own executable to two temporary paths, created one visible test window per process, and played quiet locally generated tones through Windows MediaPlayer. It did not route existing user applications. Windows exposed active sessions for both processes on their separately assigned physical endpoints. The test then moved one test window to the other display and back, applied the same monitor decision functions and native provider used by the application, and observed the corresponding endpoint changes. Original per-process role preferences were restored and the test processes and temporary files were removed.

The earlier legacy `PlaySound` version of the test accepted a preference but did not move the second stream, even after its playback restart. That failed experiment is why the UI distinguishes policy acceptance from observed output and explains stream-recreation limits. It is not counted as a successful routing test.

A final rerun initially encountered interference from the actively routing development preview. Its activity log showed that it also owned the helper processes. After pausing that preview, the final MediaPlayer test passed again, including an explicit assertion that the Windows global default output was unchanged. The preview was then resumed and its active status was observed.

The desktop executable was launched through Tauri's embedded frontend. Onboarding showed the real vertical display arrangement and both actual endpoint names. The dashboard displayed live application sessions, saved assignments and rules, and backend-confirmed activity events. The user interacted with that preview during development; its settings are preserved by the final identifier migration.

These observations establish native provider behavior for the tested Windows MediaPlayer processes and the live development session. They do not establish compatibility with every application framework or every Windows build.

## Built artifacts

The optimized Windows executable and per-user NSIS installer were built successfully. Copies are in `artifacts/`, alongside the installer updater signature and SHA-256 checksums. The installer is approximately 2.97 MiB. The public-key verifier confirmed both the artifact signature and its signed version, `0.1.0`. An altered temporary copy was rejected; the delivered installer was not changed. This verifies Tauri update-package signing, not Windows Authenticode trust.

## Remaining release tests

The GitHub repository is now [hazavi/AudioFlow](https://github.com/hazavi/AudioFlow), and its manifest endpoint is configured in source. A real installed older version downloading and installing a newer GitHub release has not been tested. The workflow still needs the signing secret and a version-tagged release before that test can run. The local installer described above was built before the repository endpoint was configured.

Windows restart/autostart, sleep/resume, hot-unplugging physical monitors and devices, protected applications, fullscreen games, and a fresh-account installer installation remain manual validation tasks. These actions were not simulated or reported as passed. No Windows Authenticode certificate is configured.

Run the hardware test with the command in [DEVELOPMENT.md](DEVELOPMENT.md). Follow [docs/releases.md](docs/releases.md) for signed-release setup and an old-to-new installation test.
