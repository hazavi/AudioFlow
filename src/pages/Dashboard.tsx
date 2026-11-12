import {
  AudioLines,
  ArrowUpRight,
  Check,
  CirclePause,
  Monitor,
  Radio,
  Volume2,
} from "lucide-react";
import { useSystem, settings } from "../services/tauri";
import { useUI } from "../stores/ui";
import { Switch } from "../components/ui/switch";
import { Button } from "../components/ui/button";
import { AppList, MonitorCard, Section, Empty } from "../components/shared";
import { time } from "../lib/utils";
import type { Application } from "../types";
export function Dashboard({
  onSelect,
}: {
  onSelect: (app: Application) => void;
}) {
  const s = useSystem((state) => state.snapshot)!;
  const busy = useSystem((state) => state.busy);
  const enabled = s.config.settings.routingEnabled;
  const active = s.applications.filter((a) => a.active);
  return (
    <>
      <div className="routing-strip">
        <div
          className={`routing-symbol ${enabled && s.routingSupported ? "on" : ""}`}
        >
          {enabled && s.routingSupported ? (
            <Radio size={21} />
          ) : (
            <CirclePause size={21} />
          )}
        </div>
        <div className="grow">
          <strong>
            Automatic routing{" "}
            {!s.routingSupported
              ? "unavailable"
              : enabled
                ? "active"
                : "paused"}
          </strong>
          <p>
            {!s.routingSupported
              ? s.compatibility
              : enabled
                ? "Application audio follows your monitor assignments."
                : "Your current audio outputs stay where they are."}
          </p>
        </div>
        <span className="label">Automatic routing</span>
        <Switch
          aria-label="Automatic routing"
          checked={enabled}
          disabled={busy || (!s.routingSupported && !enabled)}
          onCheckedChange={(value) => void settings({ routingEnabled: value })}
        />
      </div>
      <div className="metrics">
        {[
          [Monitor, s.monitors.length, "Connected monitors"],
          [Volume2, s.devices.length, "Audio outputs"],
          [AudioLines, active.length, "Active applications"],
          [Check, s.successfulChanges, "Verified switches"],
        ].map(([Icon, value, label]) => {
          const I = Icon as typeof Monitor;
          return (
            <div className="metric" key={String(label)}>
              <div className="inline">
                <I size={14} />
                <span className="label">{String(label)}</span>
              </div>
              <strong>{String(value).padStart(2, "0")}</strong>
            </div>
          );
        })}
      </div>
      <Section
        title="Monitor outputs"
        meta={`${s.monitors.length} connected`}
        actions={
          <Button
            size="sm"
            variant="ghost"
            onClick={() => useUI.getState().setPage("Monitors")}
          >
            Configure <ArrowUpRight size={13} />
          </Button>
        }
      >
        <div className="monitor-grid">
          {s.monitors.map((m, i) => (
            <MonitorCard key={m.id} monitor={m} index={i} />
          ))}
        </div>
        {!s.monitors.length && (
          <Empty
            title="Waiting for displays"
            detail="Refresh to enumerate the Windows desktop."
          />
        )}
      </Section>
      <Section
        title="Audio applications"
        meta={`${active.length} active`}
        actions={
          <Button
            size="sm"
            variant="ghost"
            onClick={() => useUI.getState().setPage("Applications")}
          >
            View all <ArrowUpRight size={13} />
          </Button>
        }
      >
        <AppList
          applications={s.applications.slice(0, 5)}
          onSelect={onSelect}
        />
      </Section>
      <Section
        title="Recent activity"
        actions={
          <Button
            size="sm"
            variant="ghost"
            onClick={() => useUI.getState().setPage("Activity")}
          >
            View history <ArrowUpRight size={13} />
          </Button>
        }
      >
        {!s.activity.length ? (
          <div className="padded muted">Routing events will appear here.</div>
        ) : (
          <div className="recent-events">
            {s.activity.slice(0, 4).map((e) => (
              <div key={e.id}>
                <time>{time(e.timestamp)}</time>
                <i
                  className={
                    e.outcome === "failed"
                      ? "error-dot"
                      : e.outcome === "verified"
                        ? "success-dot"
                        : ""
                  }
                />
                <span>
                  {e.application && <strong>{e.application} · </strong>}
                  {e.message}
                </span>
                <span className="label">{e.outcome}</span>
              </div>
            ))}
          </div>
        )}
      </Section>
    </>
  );
}
