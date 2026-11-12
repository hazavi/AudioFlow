import { useEffect, useState } from "react";
import {
  Activity as ActivityIcon,
  AppWindow,
  AudioLines,
  ChevronRight,
  CircleHelp,
  LayoutDashboard,
  LoaderCircle,
  Minus,
  Monitor,
  RefreshCw,
  Settings2,
  SlidersHorizontal,
  Square,
  X,
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { action, connect, desktop, useSystem } from "../services/tauri";
import { startUpdater } from "../services/updater";
import { useUI } from "../stores/ui";
import { ThemeToggle } from "../components/theme/ThemeToggle";
import { Button } from "../components/ui/button";
import { UpdateBanner } from "../components/updates/UpdateBanner";
import { ApplicationDetails } from "../components/ApplicationDetails";
import { Dashboard } from "../pages/Dashboard";
import { Monitors } from "../pages/Monitors";
import { Applications } from "../pages/Applications";
import { Rules } from "../pages/Rules";
import { Activity } from "../pages/Activity";
import { Settings } from "../pages/Settings";
import { Onboarding } from "../pages/Onboarding";
import type { Application, Page } from "../types";
const nav = [
  ["Dashboard", LayoutDashboard],
  ["Monitors", Monitor],
  ["Applications", AppWindow],
  ["Routing Rules", SlidersHorizontal],
  ["Activity", ActivityIcon],
  ["Settings", Settings2],
] as const;
const subtitles: Record<Page, string> = {
  Dashboard: "Your audio follows your screens.",
  Monitors: "Assign an audio output to each connected display.",
  Applications: "Control the output of each audio application.",
  "Routing Rules": "Make room for the exceptions.",
  Activity: "Device changes, output preferences, and verified audio sessions.",
  Settings: "Make AudioFlow work your way.",
};
export function App() {
  const { page, setPage } = useUI();
  const { snapshot: s, loading, busy, error, notice } = useSystem();
  const [selected, setSelected] = useState<Application | null>(null);
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let updateCleanup: (() => void) | undefined;
    let cancelled = false;
    void connect().then((fn) => {
      if (cancelled) fn();
      else cleanup = fn;
    });
    void startUpdater().then((fn) => {
      if (cancelled) fn();
      else updateCleanup = fn;
    });
    return () => {
      cancelled = true;
      cleanup?.();
      updateCleanup?.();
    };
  }, []);
  useEffect(() => {
    if (!desktop) return;
    const w = getCurrentWindow();
    let off: (() => void) | undefined;
    let cancelled = false;
    void w.isMaximized().then(setMaximized);
    void w
      .onResized(async () => setMaximized(await w.isMaximized()))
      .then((fn) => {
        if (cancelled) fn();
        else off = fn;
      });
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);
  async function windowAction(kind: "minimize" | "toggleMaximize" | "close") {
    if (desktop)
      try {
        await getCurrentWindow()[kind]();
      } catch (e) {
        useSystem.setState({ error: String(e) });
      }
  }
  const enabled = s?.config.settings.routingEnabled && s.routingSupported;
  const routingState =
    !s || !s.routingSupported ? "UNAVAILABLE" : enabled ? "ACTIVE" : "PAUSED";
  return (
    <div className={`app-shell ${maximized ? "maximized" : ""}`}>
      <header className="titlebar">
        <div className="brand" data-tauri-drag-region>
          <img src="/icon.svg" alt="" width="26" height="26" />
          <strong>AUDIOFLOW</strong>
          <span>AUDIO ROUTING UTILITY</span>
        </div>
        <div className="drag-region" data-tauri-drag-region />
        <div className="title-tools">
          <span className={`title-status ${enabled ? "success" : "muted"}`}>
            <i />
            {s ? `ROUTING ${routingState}` : "CONNECTING"}
          </span>
          <ThemeToggle />
          <Button
            size="icon"
            variant="ghost"
            aria-label="Settings"
            onClick={() => setPage("Settings")}
          >
            <Settings2 size={16} />
          </Button>
          <span className="title-divider" />
          <button
            className="window-button"
            aria-label="Minimize"
            onClick={() => void windowAction("minimize")}
          >
            <Minus size={14} />
          </button>
          <button
            className="window-button"
            aria-label={maximized ? "Restore window" : "Maximize"}
            onClick={() => void windowAction("toggleMaximize")}
          >
            <Square size={11} />
          </button>
          <button
            className="window-button close"
            aria-label={
              s?.config.settings.minimizeToTray
                ? "Close to tray"
                : "Close AudioFlow"
            }
            onClick={() => void windowAction("close")}
          >
            <X size={16} />
          </button>
        </div>
      </header>
      <div className="workspace">
        <aside className="sidebar">
          <div className="sidebar-label label">Workspace</div>
          <nav aria-label="Main navigation">
            {nav.map(([label, Icon]) => (
              <button
                key={label}
                className={page === label ? "active" : ""}
                aria-current={page === label ? "page" : undefined}
                onClick={() => setPage(label)}
              >
                <Icon size={16} />
                <span>{label}</span>
                {page === label && <ChevronRight size={12} />}
              </button>
            ))}
          </nav>
          <div className="sidebar-bottom">
            <div className="local-badge">
              <span className={`status-dot ${enabled ? "on" : ""}`} />
              <div>
                <strong>
                  {enabled ? "Routing is active" : "Routing is paused"}
                </strong>
                <small>Runs locally on Windows</small>
              </div>
            </div>
            <button
              className="sidebar-help"
              onClick={() => setPage("Settings")}
            >
              <CircleHelp size={14} /> About & compatibility
            </button>
            <div className="sidebar-version label">
              AudioFlow {s ? `v${s.version}` : ""}
              <AudioLines size={15} />
            </div>
          </div>
        </aside>
        <main>
          <div className="page-header">
            <div>
              <div className="eyebrow label">
                AudioFlow <ChevronRight size={11} /> {page}
              </div>
              <h1>{page}</h1>
              <p>{subtitles[page]}</p>
            </div>
            <Button
              disabled={busy || !s}
              onClick={() => void action("scan_now")}
            >
              <RefreshCw size={14} className={busy ? "spin" : ""} /> Refresh
            </Button>
          </div>
          <UpdateBanner />
          {error && (
            <div className="message error-message" role="alert">
              <span>{error}</span>
              <Button
                size="icon"
                variant="ghost"
                aria-label="Dismiss error"
                onClick={() => useSystem.setState({ error: null })}
              >
                <X size={14} />
              </Button>
            </div>
          )}
          {notice && (
            <div className="message notice-message" role="status">
              <span>{notice}</span>
              <Button
                size="icon"
                variant="ghost"
                aria-label="Dismiss notification"
                onClick={() => useSystem.setState({ notice: null })}
              >
                <X size={14} />
              </Button>
            </div>
          )}
          {s?.errors.map((e) => (
            <div className="message error-message" role="alert" key={e}>
              {e}
            </div>
          ))}
          {loading && (
            <div className="loading">
              <LoaderCircle size={24} className="spin" />
              <span>Connecting to Windows audio services…</span>
            </div>
          )}
          {s && (
            <div className="page-content" key={page}>
              {page === "Dashboard" ? (
                <Dashboard onSelect={setSelected} />
              ) : page === "Monitors" ? (
                <Monitors />
              ) : page === "Applications" ? (
                <Applications onSelect={setSelected} />
              ) : page === "Routing Rules" ? (
                <Rules />
              ) : page === "Activity" ? (
                <Activity />
              ) : (
                <Settings />
              )}
            </div>
          )}
        </main>
      </div>
      <footer className="statusbar">
        <span>
          <i className={enabled ? "success-dot" : ""} /> ROUTING: {routingState}
        </span>
        <span>MONITORS: {s?.monitors.length ?? "—"}</span>
        <span>AUDIO DEVICES: {s?.devices.length ?? "—"}</span>
        <span className="grow" />
        <span>{s ? `AUDIOFLOW v${s.version}` : "NATIVE WINDOWS UTILITY"}</span>
      </footer>
      {selected && s && (
        <ApplicationDetails
          key={selected.pid}
          application={selected}
          onClose={() => setSelected(null)}
        />
      )}
      {s && !s.config.settings.onboardingCompleted && s.errors.length === 0 && (
        <Onboarding />
      )}
    </div>
  );
}
