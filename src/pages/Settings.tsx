import { useRef, useState, type ReactNode } from "react";
import { Download, ExternalLink, RotateCcw, Upload } from "lucide-react";
import { action, exportText, settings, useSystem } from "../services/tauri";
import { checkUpdates } from "../services/updater";
import { useUpdateStore } from "../stores/useUpdateStore";
import { Button } from "../components/ui/button";
import { Switch } from "../components/ui/switch";
import { Dialog } from "../components/ui/dialog";
import { DeviceSelect, Section } from "../components/shared";
import { UpdateStatus } from "../components/updates/UpdateStatus";
import { download } from "../lib/utils";
import type { Settings as UserSettings, Theme } from "../types";
function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="setting-row">
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}
export function Settings() {
  const s = useSystem((state) => state.snapshot)!;
  const busy = useSystem((state) => state.busy);
  const update = useUpdateStore();
  const [reset, setReset] = useState(false);
  const [importing, setImporting] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const [delay, setDelay] = useState(s.config.settings.switchDelayMs);
  const config = s.config.settings;
  function toggle(key: keyof UserSettings, title: string, description: string) {
    return (
      <SettingRow key={key} title={title} description={description}>
        <Switch
          aria-label={title}
          checked={Boolean(config[key])}
          disabled={busy}
          onCheckedChange={(value) => void settings({ [key]: value })}
        />
      </SettingRow>
    );
  }
  async function exportConfig() {
    await action("export_file", { kind: "config" });
  }
  return (
    <div className="settings-stack">
      <Section title="General">
        {toggle(
          "launchOnStartup",
          "Launch with Windows",
          "Start AudioFlow when you sign in to your Windows account.",
        )}
        {toggle(
          "startMinimized",
          "Start in the system tray",
          "Keep the window hidden on startup after setup is complete.",
        )}
        {toggle(
          "minimizeToTray",
          "Close to system tray",
          "Keep routing active when you close the main window.",
        )}
        {toggle(
          "notifications",
          "Desktop notifications",
          "Notify you about routing errors while AudioFlow is hidden.",
        )}
      </Section>
      <Section title="Appearance">
        <SettingRow
          title="Theme"
          description="System follows the Windows color preference in real time."
        >
          <select
            aria-label="Theme"
            value={config.theme}
            disabled={busy}
            onChange={(e) => void settings({ theme: e.target.value as Theme })}
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="system">System</option>
          </select>
        </SettingRow>
      </Section>
      <Section title="Audio routing">
        {toggle(
          "routingEnabled",
          "Automatic routing",
          "Follow your monitor assignments unless a rule takes priority.",
        )}
        <SettingRow
          title="Switching delay"
          description="Wait after a monitor change to prevent rapid switching."
        >
          <form
            className="inline"
            onSubmit={(e) => {
              e.preventDefault();
              void settings({ switchDelayMs: delay });
            }}
          >
            <input
              aria-label="Switching delay in milliseconds"
              type="number"
              min={100}
              max={5000}
              step={50}
              required
              value={delay}
              onChange={(e) => setDelay(Number(e.target.value))}
            />
            <span className="label">ms</span>
            <Button size="sm" disabled={busy || delay === config.switchDelayMs}>
              Save
            </Button>
          </form>
        </SettingRow>
        {toggle(
          "routeNewApps",
          "Route new audio applications",
          "Apply monitor routing when a new audio process is detected.",
        )}
        {toggle(
          "respectManual",
          "Respect Windows manual outputs",
          "Leave Volume Mixer choices alone unless an explicit application rule grants control.",
        )}
        {toggle(
          "restoreOnExit",
          "Restore managed outputs on exit",
          "Restore prior preferences only if AudioFlow still owns the current route.",
        )}
        <SettingRow
          title="Device reconnection"
          description="Choose what happens when an assigned speaker disconnects."
        >
          <select
            aria-label="Device reconnection"
            value={config.reconnectBehavior}
            onChange={(e) =>
              void settings({
                reconnectBehavior: e.target.value as "wait" | "fallback",
              })
            }
            disabled={busy}
          >
            <option value="wait">Wait for assigned output</option>
            <option value="fallback">Use fallback output</option>
          </select>
        </SettingRow>
        <SettingRow
          title="Fallback output"
          description="Used only when fallback behavior is selected."
        >
          <DeviceSelect
            value={config.fallbackEndpoint || ""}
            devices={s.devices}
            onChange={(value) =>
              void settings({ fallbackEndpoint: value || null })
            }
            disabled={busy}
            label="Fallback output"
          />
        </SettingRow>
      </Section>
      <Section title="Updates" meta={`v${s.version}`}>
        {!s.updaterConfigured && (
          <div className="settings-warning">
            This build has no configured release source. Configure the GitHub
            repository and public signing key before distributing it. See the
            release setup guide.
          </div>
        )}
        {toggle(
          "automaticUpdates",
          "Automatically check for updates",
          "Check at startup and every 5 minutes, including in the tray. Retry failed checks after a minute.",
        )}
        {toggle(
          "updateNotifications",
          "Notify me about updates",
          "Show the update banner when a newer version is available.",
        )}
        <SettingRow
          title="Check for updates"
          description={`Last checked: ${update.lastChecked ? new Date(update.lastChecked).toLocaleString() : "Never"}${update.version ? ` · Latest: v${update.version}` : ""}`}
        >
          <Button
            disabled={
              !s.updaterConfigured ||
              ["checking", "downloading", "installing"].includes(update.status)
            }
            onClick={() => void checkUpdates(true)}
          >
            {update.status === "checking" ? "Checking…" : "Check now"}
          </Button>
        </SettingRow>
        <div className="setting-row">
          <div>
            <UpdateStatus />
            {update.error && <p className="danger">{update.error}</p>}
          </div>
          <Button
            size="sm"
            disabled={!s.repository}
            onClick={() => void action("open_releases")}
          >
            View releases <ExternalLink size={13} />
          </Button>
        </div>
      </Section>
      <Section title="Advanced">
        {toggle(
          "debugLogging",
          "Debug logging",
          "Write diagnostic details to a bounded local log.",
        )}
        <SettingRow title="Routing compatibility" description={s.compatibility}>
          <span
            className={`badge ${s.routingSupported ? "success" : "warning"}`}
          >
            {s.routingSupported ? "AVAILABLE" : "UNAVAILABLE"}
          </span>
        </SettingRow>
        <SettingRow
          title="Refresh Windows devices"
          description="Reconcile displays, output endpoints, windows, and audio sessions."
        >
          <Button disabled={busy} onClick={() => void action("scan_now")}>
            Scan now
          </Button>
        </SettingRow>
        <SettingRow
          title="Configuration"
          description="Back up or import your settings, assignments, and application rules."
        >
          <Button size="sm" onClick={() => void exportConfig()}>
            <Download size={13} /> Export
          </Button>
          <Button size="sm" onClick={() => input.current?.click()}>
            <Upload size={13} /> Import
          </Button>
          <input
            hidden
            type="file"
            accept=".json,application/json"
            ref={input}
            onChange={async (e) => {
              const file = e.target.files?.[0];
              if (file) {
                if (file.size > 2_000_000)
                  useSystem.setState({ error: "Configuration exceeds 2 MB." });
                else setImporting(await file.text());
              }
              e.target.value = "";
            }}
          />
        </SettingRow>
        <SettingRow
          title="Reset AudioFlow"
          description="Remove assignments and rules and return to first-time setup."
        >
          <Button size="sm" onClick={() => setReset(true)}>
            <RotateCcw size={13} /> Reset settings
          </Button>
        </SettingRow>
      </Section>
      <Section title="About AudioFlow">
        <div className="about">
          <img src="/icon.svg" width="48" height="48" alt="" />
          <div>
            <strong>AudioFlow v{s.version}</strong>
            <p>Your sound. Your screen. Automatically.</p>
            <small className="muted">
              Local Windows audio utility · MIT license · React, Tauri, and
              windows-rs
            </small>
          </div>
        </div>
      </Section>
      <Dialog
        open={reset}
        onOpenChange={setReset}
        title="Reset AudioFlow settings?"
        description="This removes saved monitor assignments and application rules. Export a backup first if you want to keep them."
      >
        <div className="dialog-body dialog-actions">
          <Button onClick={() => setReset(false)}>Cancel</Button>
          <Button
            variant="destructive"
            disabled={busy}
            onClick={async () => {
              if (await action("reset_config")) setReset(false);
            }}
          >
            Reset settings
          </Button>
        </div>
      </Dialog>
      <Dialog
        open={importing !== null}
        onOpenChange={(v) => !v && setImporting(null)}
        title="Import configuration?"
        description="This replaces your current assignments, rules, and settings. Imported routing and Windows startup settings will take effect."
      >
        <div className="dialog-body dialog-actions">
          <Button onClick={() => setImporting(null)}>Cancel</Button>
          <Button
            variant="default"
            disabled={busy}
            onClick={async () => {
              if (await action("import_config", { json: importing }))
                setImporting(null);
            }}
          >
            Import configuration
          </Button>
        </div>
      </Dialog>
    </div>
  );
}
