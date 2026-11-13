# Development

## Windows prerequisites

Use Node.js 24, stable Rust with the MSVC target, Microsoft C++ Build Tools, a Windows SDK, and WebView2. The official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) describe these tools. This workspace's Rust installation is under `.tools`; it is ignored by Git and has not changed the global PATH.

```powershell
npm ci
npm run desktop
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
npm run tauri -- build --bundles nsis
```

Use `scripts/desktop.ps1` for the isolated Rust installation. Run a read-only system probe with:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --example diagnostics
```

The opt-in hardware test plays quiet tones through two real outputs and creates its own temporary windows and executables. It does not route user applications. It needs a normal Windows user token; the development filesystem sandbox denied audio-policy writes.

Pause any running copy of AudioFlow or another automatic audio router before running this isolated provider test, then restore its prior state afterward. Otherwise the two engines can compete over the test processes and produce a false failure. Pausing AudioFlow preserves existing outputs.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib native::windows::hardware::two_output_routing -- --ignored --nocapture
```

## Native architecture

One dedicated worker owns its COM apartment, endpoint enumerator, policy object, and notification registration. A separate WinEvent message thread delivers foreground, visibility, and location events. All native IPC mutations use this worker through channels; the Tauri UI thread does not enumerate Windows devices or audio sessions. A three-second reconciliation scan covers display hot-plug, session creation, missed hooks, and process exits. Pending switches and verification use a 150 ms cadence, with configurable stable-target delay and a ten-percent window-area hysteresis margin.

Window selection stays with the previous eligible window before considering focus or area. Browser audio children share that selection through the window owner's PID and creation time. Session gaps preserve route ownership and window tracking while the processes remain alive; they do not restore old preferences. The restore-on-exit setting controls restoration when AudioFlow shuts down. Tool windows are excluded from routing candidates.

Monitor hysteresis uses the same owner identity, so a newly created audio child cannot pick a competing output near a display boundary. Application output labels use active sessions when playback is active, deduplicate endpoints, and sort their IDs. Inactive session history remains available in application details. Update checks start when configuration arrives, repeat every five minutes using native wake events and a frontend timer, and retry failures after one minute.

All desktop coordinates are read with per-monitor DPI awareness. GDI provides physical bounds and persistent monitor interface paths; QueryDisplayConfig provides friendly names. Device endpoints are independent of display order. Friendly-name suggestions are advisory only.

The provider reads and writes the render preference for console, multimedia, and communications roles. It captures the prior values, rolls back a partial setter failure, reads preferences back, and reports policy acceptance separately from observed live audio. There is no global-default setter and no clear-all operation. Endpoint test playback uses a shared-mode WASAPI client on the selected IMMDevice and a locally generated envelope-limited 350 ms tone.

PID ownership includes process creation time and executable path. Restoration checks both identity and the last applied preferences. A user change takes priority. Rules match an exact executable filename or full path case-insensitively. Priority ties use mode precedence, then stable rule ID. Browser child processes only inherit windows through ancestors with the same executable path; unrelated launcher windows never own a stream.

Configuration has a schema version and strict validation. Invalid configuration is preserved and locked against ordinary saves until explicit reset or import. Writes use a synced temporary file and atomic replacement. Assignments are retained for disconnected devices. A one-time migration preserves settings from the initial development identifier. Activity is limited to 300 entries and flushed at most every 30 seconds, with a final flush on shutdown. Debug logging is controlled separately.

A bounded local ownership journal records the exact prior role preferences and the last values AudioFlow wrote. Recovery only reclaims an exact executable when Windows still exposes those applied values and the earlier process identity has ended. This lets AudioFlow resume its own preferences without treating them as unrelated Volume Mixer choices after a restart. Exited processes cannot be safely reset through the PID-based policy API; their journal entries are retained for later recovery instead of changing registry-wide audio state.

The frontend subscribes before requesting its initial snapshot. Backend snapshots remain authoritative. No browser-only demo data is generated; opening the Vite URL without Tauri displays a native-services error. UI preferences use Zustand persistence for first-paint theme selection; backend settings are then applied. System mode subscribes to `prefers-color-scheme` changes.

## Updater lifecycle

Native scheduling sends checks at startup and every 12 hours; the shared service is mounted at the application root and remains alive in the tray. It avoids overlapping checks and suppresses redundant automatic requests. GitHub's latest manifest provides stable releases; prerelease versions are also rejected locally. Download completion does not imply successful installation. The user confirms installation separately after download. Before installation, the routing worker restores eligible owned preferences, stops event hooks, flushes history, and releases COM objects. Installation failure recreates the worker. Native Tauri signature checks verify downloaded bytes and the version bound into the signature.

Refer to [docs/releases.md](docs/releases.md) for signing and publishing. Do not commit private keys or put them in frontend environment variables.

## Design references

The supplied outline, spacing, color, and motion requirements define the UI. No ArtCraft screenshot was present in the supplied attachments, so its exact composition could not be compared. The interface uses shadcn-style Radix primitives with the new-york conventions and shared semantic Tailwind variables. Origin UI's [theme dropdown on 21st.dev](https://21st.dev/@originui/components/dropdown-menu/theme-toggle-with-dropdown) was reviewed and adapted with persistence and live system theme tracking. Other community components were not pulled in merely for decoration.

The [MonitorAudioRouter reference](https://github.com/TechlyAccurate/MonitorAudioRouter) was studied for behavior and browser limitations. Its repository did not expose a license in the inspected root; none of its code was reused. The policy ABI was checked against the separately licensed EarTrumpet interface declarations. This implementation has its own Rust worker, data model, ownership checks, decision engine, and frontend.
