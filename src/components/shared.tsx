import type { ReactNode } from "react";
import {
  Monitor as MonitorIcon,
  Volume2,
  AudioLines,
  AlertCircle,
  ChevronRight,
} from "lucide-react";
import { action, useSystem } from "../services/tauri";
import { useUI } from "../stores/ui";
import { Button } from "./ui/button";
import type { Application, Device, Monitor } from "../types";
export function Label({ children }: { children: ReactNode }) {
  return <span className="label">{children}</span>;
}
export function Empty({ title, detail }: { title: string; detail: string }) {
  return (
    <div className="empty">
      <AudioLines size={24} />
      <strong>{title}</strong>
      <p>{detail}</p>
    </div>
  );
}
export function Section({
  title,
  meta,
  children,
  actions,
}: {
  title: string;
  meta?: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <section className="panel">
      <div className="section-heading">
        <div className="inline">
          <h2>{title}</h2>
          {meta && <span className="count">{meta}</span>}
        </div>
        {actions}
      </div>
      {children}
    </section>
  );
}
export function DeviceSelect({
  value,
  onChange,
  devices,
  unassigned = true,
  disabled = false,
  label = "Audio output",
}: {
  value: string;
  onChange: (id: string) => void;
  devices: Device[];
  unassigned?: boolean;
  disabled?: boolean;
  label?: string;
}) {
  return (
    <select
      aria-label={label}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
    >
      {unassigned && <option value="">Select an output…</option>}
      <option value="default">Default Windows audio</option>
      {value && value !== "default" && !devices.some((d) => d.id === value) && (
        <option value={value}>Disconnected output · {value}</option>
      )}
      {devices.map((d) => (
        <option key={d.id} value={d.id}>
          {d.name}
          {d.isDefault ? " · Windows default" : ""}
        </option>
      ))}
    </select>
  );
}
export function MonitorArrangement({
  monitors,
  compact = false,
}: {
  monitors: Monitor[];
  compact?: boolean;
}) {
  if (!monitors.length)
    return (
      <Empty
        title="No displays detected"
        detail="Connect a display and refresh to scan Windows again."
      />
    );
  const left = Math.min(...monitors.map((m) => m.bounds.x));
  const top = Math.min(...monitors.map((m) => m.bounds.y));
  const width =
    Math.max(...monitors.map((m) => m.bounds.x + m.bounds.width)) - left;
  const height =
    Math.max(...monitors.map((m) => m.bounds.y + m.bounds.height)) - top;
  return (
    <div className={`arrangement ${compact ? "compact" : ""}`}>
      <div
        className="desktop-map"
        style={{
          aspectRatio: `${Math.max(width, 1)} / ${Math.max(height, 1)}`,
          maxHeight: compact ? 120 : 210,
        }}
      >
        {monitors.map((m, i) => (
          <div
            className={`display-block ${m.primary ? "primary" : ""}`}
            key={m.id}
            title={`${m.name} · ${m.bounds.x}, ${m.bounds.y}`}
            style={{
              left: `${((m.bounds.x - left) / width) * 100}%`,
              top: `${((m.bounds.y - top) / height) * 100}%`,
              width: `${(m.bounds.width / width) * 100}%`,
              height: `${(m.bounds.height / height) * 100}%`,
            }}
          >
            <span>{String(i + 1).padStart(2, "0")}</span>
            <small>{m.primary ? "PRIMARY" : "DISPLAY"}</small>
          </div>
        ))}
      </div>
      <span className="map-caption label">
        Windows desktop arrangement · physical coordinates
      </span>
    </div>
  );
}
export function MonitorCard({
  monitor: m,
  index,
  details = false,
}: {
  monitor: Monitor;
  index: number;
  details?: boolean;
}) {
  const snapshot = useSystem((s) => s.snapshot)!;
  const busy = useSystem((s) => s.busy);
  const assigned = snapshot.config.assignments[m.id] || "";
  const available =
    assigned === "default" || snapshot.devices.some((d) => d.id === assigned);
  const suggested =
    m.name.length > 4 && !/generic|unknown/i.test(m.name)
      ? snapshot.devices.filter((d) =>
          d.name.toLowerCase().includes(m.name.toLowerCase()),
        )
      : [];
  return (
    <article className="monitor-card">
      <div className="monitor-card-top">
        <Label>Monitor {String(index + 1).padStart(2, "0")}</Label>
        <span
          className={`badge ${assigned && available ? "success" : "warning"}`}
        >
          {!assigned
            ? "UNASSIGNED"
            : available
              ? "CONNECTED"
              : "OUTPUT OFFLINE"}
        </span>
      </div>
      <div className="monitor-info">
        <div className={`monitor-symbol ${m.primary ? "is-primary" : ""}`}>
          <MonitorIcon size={37} strokeWidth={1.1} />
          <span>{index + 1}</span>
        </div>
        <div>
          <h3>{m.name}</h3>
          <p>
            {m.bounds.width} × {m.bounds.height}
            {m.refreshRate ? ` · ${m.refreshRate} Hz` : ""}
          </p>
          <Label>{m.primary ? "Primary display" : "Secondary display"}</Label>
        </div>
      </div>
      {details && (
        <dl className="technical">
          <div>
            <dt>Position</dt>
            <dd>
              {m.bounds.x}, {m.bounds.y}
            </dd>
          </div>
          <div>
            <dt>Windows name</dt>
            <dd>{m.deviceName}</dd>
          </div>
          <div>
            <dt>Persistent ID</dt>
            <dd className="break-all">{m.id}</dd>
          </div>
        </dl>
      )}
      <div className="assignment">
        <label htmlFor={`output-${index}`}>
          <Volume2 size={13} /> Audio output
        </label>
        <div id={`output-${index}`}>
          <DeviceSelect
            label={`Output for monitor ${index + 1}`}
            value={assigned}
            devices={snapshot.devices}
            disabled={busy}
            onChange={(id) =>
              void action(
                "set_monitor_assignment",
                { monitorId: m.id, endpointId: id || null },
                "Monitor assignment saved.",
              )
            }
          />
        </div>
      </div>
      {!assigned && suggested.length === 1 && (
        <Button
          className="suggestion"
          size="sm"
          variant="ghost"
          disabled={busy}
          onClick={() =>
            void action("set_monitor_assignment", {
              monitorId: m.id,
              endpointId: suggested[0].id,
            })
          }
        >
          Suggested: {suggested[0].name}
        </Button>
      )}
      <div className="monitor-card-bottom">
        <span className="muted small">
          {!assigned
            ? "Choose an output to enable routing"
            : "Assignment saved locally"}
        </span>
        <Button
          size="sm"
          variant="ghost"
          disabled={!assigned || !available || busy}
          onClick={() =>
            void action(
              "test_audio_device",
              { endpointId: assigned },
              "Test sound finished.",
            )
          }
        >
          <Volume2 size={13} /> Test
        </Button>
      </div>
    </article>
  );
}
export function AppList({
  applications,
  onSelect,
}: {
  applications: Application[];
  onSelect: (app: Application) => void;
}) {
  const s = useSystem((state) => state.snapshot)!;
  if (!applications.length)
    return (
      <Empty
        title="No audio applications"
        detail="Start playback in an application, then refresh. Idle sessions appear when Windows exposes them."
      />
    );
  return (
    <div className="app-list">
      <div className="app-row app-table-head label">
        <span>Application</span>
        <span>Monitor</span>
        <span>Audio output</span>
        <span>Status</span>
        <span />
      </div>
      {applications.map((a) => (
        <button key={a.pid} className="app-row" onClick={() => onSelect(a)}>
          <span className="app-identity">
            <span className={`app-icon ${a.active ? "playing" : ""}`}>
              {a.iconData ? (
                <img src={a.iconData} width={22} height={22} alt="" />
              ) : (
                <AudioLines size={18} />
              )}
            </span>
            <span>
              <strong>{a.name}</strong>
              <small>
                PID {a.pid} ·{" "}
                {a.mode === "auto"
                  ? "Follow monitor"
                  : a.mode === "fixed"
                    ? "Fixed device"
                    : a.mode === "ignore"
                      ? "Ignored"
                      : "Windows default"}
              </small>
            </span>
          </span>
          <span className="muted">
            {s.monitors.findIndex((m) => m.id === a.monitorId) >= 0
              ? `Monitor ${String(s.monitors.findIndex((m) => m.id === a.monitorId) + 1).padStart(2, "0")}`
              : "No eligible window"}
          </span>
          <span className="truncate">
            {a.active
              ? a.endpointIds
                  .map((id) => s.devices.find((d) => d.id === id)?.name || id)
                  .join(", ") || "Not observed"
              : "No active playback"}
          </span>
          <span>
            <span className={`audio-status ${a.active ? "success" : "muted"}`}>
              <i />
              {a.active ? "Active stream" : "Idle"}
            </span>
            {a.lastError && <AlertCircle size={12} className="danger" />}
          </span>
          <ChevronRight size={14} className="muted" />
        </button>
      ))}
    </div>
  );
}
export function BrowserNotice() {
  return (
    <div className="info-note">
      <AlertCircle size={15} />
      <p>
        Windows routes audio per process. Browser windows and tabs that share an
        audio process share an output. AudioFlow keeps following the same
        eligible window. Move that window to change its output; changing focus
        alone keeps audio on its current screen.
      </p>
    </div>
  );
}
export function GoSettings() {
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => useUI.getState().setPage("Settings")}
    >
      Settings <ChevronRight size={13} />
    </Button>
  );
}
