import { useState } from "react";
import { Copy, Download, Search, Trash2 } from "lucide-react";
import { action, exportText, useSystem } from "../services/tauri";
import { Button } from "../components/ui/button";
import { Dialog } from "../components/ui/dialog";
import { Empty, Section } from "../components/shared";
import { download, time } from "../lib/utils";
export function Activity() {
  const s = useSystem((state) => state.snapshot)!;
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const [clear, setClear] = useState(false);
  const events = s.activity.filter(
    (e) =>
      `${e.application || ""} ${e.message}`
        .toLowerCase()
        .includes(query.toLowerCase()) &&
      (filter === "all" || e.eventType === filter),
  );
  async function exportLogs() {
    await action("export_file", { kind: "activity" });
  }
  async function copy() {
    try {
      await navigator.clipboard.writeText(
        JSON.stringify(
          {
            version: s.version,
            compatibility: s.compatibility,
            monitors: s.monitors,
            devices: s.devices,
            errors: s.errors,
            activity: s.activity,
          },
          null,
          2,
        ),
      );
      useSystem.setState({ notice: "Diagnostics copied to clipboard." });
    } catch (e) {
      useSystem.setState({ error: `Could not copy diagnostics: ${e}` });
    }
  }
  return (
    <>
      <div className="toolbar">
        <div className="search">
          <Search size={15} />
          <input
            aria-label="Search activity"
            placeholder="Search events…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <select
          aria-label="Event type"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          <option value="all">All event types</option>
          {[...new Set(s.activity.map((e) => e.eventType))].map((type) => (
            <option key={type}>{type}</option>
          ))}
        </select>
        <Button
          size="icon"
          title="Copy diagnostics"
          aria-label="Copy diagnostics"
          onClick={() => void copy()}
        >
          <Copy size={14} />
        </Button>
        <Button
          size="icon"
          title="Export logs"
          aria-label="Export logs"
          onClick={() => void exportLogs()}
        >
          <Download size={14} />
        </Button>
        <Button
          size="icon"
          title="Clear history"
          aria-label="Clear history"
          onClick={() => setClear(true)}
        >
          <Trash2 size={14} />
        </Button>
      </div>
      <Section title="Event history" meta={`${events.length} / 300 entries`}>
        {!events.length ? (
          <Empty
            title="No matching events"
            detail="Device detection, routing decisions, and errors are recorded here. Window titles and URLs are never collected."
          />
        ) : (
          <div className="timeline">
            {events.map((e) => (
              <article key={e.id}>
                <time title={new Date(e.timestamp).toLocaleString()}>
                  {time(e.timestamp)}
                </time>
                <div>
                  <div className="inline">
                    <span
                      className={`badge ${e.outcome === "failed" ? "danger" : e.outcome === "verified" ? "success" : ""}`}
                    >
                      {e.eventType}
                    </span>
                    {e.application && <strong>{e.application}</strong>}
                    <span className="label">{e.outcome}</span>
                  </div>
                  <p>{e.message}</p>
                  {(e.previousDevice || e.newDevice) && (
                    <small className="muted">
                      {s.devices.find((d) => d.id === e.previousDevice)?.name ||
                        e.previousDevice ||
                        "Windows default"}{" "}
                      →{" "}
                      {s.devices.find((d) => d.id === e.newDevice)?.name ||
                        e.newDevice ||
                        "Windows default"}
                    </small>
                  )}
                </div>
              </article>
            ))}
          </div>
        )}
      </Section>
      <Dialog
        open={clear}
        onOpenChange={setClear}
        title="Clear activity history?"
        description="This removes AudioFlow's local event history. Your assignments and rules are preserved."
      >
        <div className="dialog-body dialog-actions">
          <Button onClick={() => setClear(false)}>Cancel</Button>
          <Button
            variant="destructive"
            onClick={async () => {
              if (await action("clear_activity")) setClear(false);
            }}
          >
            Clear history
          </Button>
        </div>
      </Dialog>
    </>
  );
}
