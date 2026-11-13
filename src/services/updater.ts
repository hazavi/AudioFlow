import {
  check,
  type Update,
  type DownloadEvent,
} from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useUpdateStore as store } from "../stores/useUpdateStore";
import { desktop, useSystem } from "./tauri";
let candidate: Update | null = null;
let checking = false;
let downloading = false;
const CHECK_INTERVAL = 5 * 60 * 1000;
const RETRY_INTERVAL = 60 * 1000;
export function progress(event: DownloadEvent) {
  if (event.event === "Started")
    store.setState({ downloaded: 0, total: event.data.contentLength ?? null });
  if (event.event === "Progress")
    store.setState((s) => ({
      downloaded: s.downloaded + event.data.chunkLength,
    }));
  // Finished is a download event, not proof that installation succeeded.
}
export async function checkUpdates(manual = false) {
  const snapshot = useSystem.getState().snapshot;
  if (!snapshot && !manual) return;
  if (!desktop || !snapshot?.updaterConfigured) {
    store.setState({
      status: "unconfigured",
      error:
        "A GitHub release repository and public signing key must be configured for this build.",
    });
    return;
  }
  if (
    (!manual && !snapshot.config.settings.automaticUpdates) ||
    checking ||
    downloading ||
    ["ready", "installing", "restarting"].includes(store.getState().status)
  )
    return;
  const last = store.getState().lastChecked;
  const interval =
    store.getState().status === "failed" ? RETRY_INTERVAL : CHECK_INTERVAL;
  if (!manual && last && Date.now() - new Date(last).getTime() < interval)
    return;
  checking = true;
  store.setState({
    status: store.getState().status === "available" ? "available" : "checking",
    error: null,
  });
  try {
    const update = await check({
      timeout: 20000,
      headers: { "Cache-Control": "no-cache" },
    });
    if (candidate) await candidate.close();
    candidate = update;
    // GitHub's latest endpoint excludes prereleases; reject them defensively as well.
    if (update && !update.version.includes("-")) {
      const changed = store.getState().version !== update.version;
      store.setState({
        status: "available",
        version: update.version,
        notes: update.body || "",
        manualCheck: manual,
        lastChecked: new Date().toISOString(),
        ...(changed || manual ? { dismissed: false, laterUntil: 0 } : {}),
      });
    } else {
      if (candidate) await candidate.close();
      candidate = null;
      store.setState({
        status: "upToDate",
        lastChecked: new Date().toISOString(),
        version: null,
      });
    }
  } catch (e) {
    store.setState({
      status: "failed",
      error: String(e),
      lastChecked: new Date().toISOString(),
    });
  } finally {
    checking = false;
  }
}
export async function downloadUpdate() {
  if (!candidate || downloading) return;
  downloading = true;
  store.setState({
    status: "downloading",
    error: null,
    downloaded: 0,
    total: null,
  });
  try {
    await candidate.download(progress);
    store.setState({ status: "ready" });
  } catch (e) {
    store.setState({ status: "failed", error: String(e) });
  } finally {
    downloading = false;
  }
}
export async function installUpdate() {
  if (!candidate || store.getState().status !== "ready") return;
  store.setState({ status: "installing", error: null });
  try {
    await invoke("prepare_update");
    await candidate.install();
    // Windows normally exits inside install(); this handles platforms where it returns.
    store.setState({ status: "restarting" });
    await relaunch();
  } catch (e) {
    store.setState({ status: "failed", error: String(e) });
    try {
      await invoke("resume_after_update");
    } catch {
      /* Keep the updater error visible. */
    }
  }
}
export async function applyUpdate() {
  if (checking || downloading) return;
  if (store.getState().status === "available") await downloadUpdate();
  if (store.getState().status === "ready") await installUpdate();
}
export async function startUpdater() {
  if (!desktop) return () => {};
  const run = () => void checkUpdates();
  let ready = false;
  const checkWhenReady = () => {
    const snapshot = useSystem.getState().snapshot;
    const next = !!(
      snapshot?.updaterConfigured && snapshot.config.settings.automaticUpdates
    );
    if (next && !ready) run();
    ready = next;
  };
  const unsubscribe = useSystem.subscribe(checkWhenReady);
  window.addEventListener("focus", run);
  window.addEventListener("online", run);
  // Native ticks work while the webview is hidden. The frontend timer also
  // covers a missed event; checkUpdates serializes and throttles both sources.
  const timer = setInterval(run, RETRY_INTERVAL);
  checkWhenReady();
  const stop = await listen("check-for-updates", run).catch(() => () => {});
  return () => {
    clearInterval(timer);
    unsubscribe();
    window.removeEventListener("focus", run);
    window.removeEventListener("online", run);
    stop();
  };
}
