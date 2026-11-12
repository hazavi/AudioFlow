import { useState } from "react";
import { Dialog } from "./ui/dialog";
import { Button } from "./ui/button";
import { BrowserNotice, DeviceSelect } from "./shared";
import { saveRule, useSystem } from "../services/tauri";
import { time } from "../lib/utils";
import type { Application, RoutingMode } from "../types";
export function ApplicationDetails({
  application,
  onClose,
}: {
  application: Application;
  onClose: () => void;
}) {
  const s = useSystem((state) => state.snapshot)!;
  const busy = useSystem((state) => state.busy);
  const current =
    s.applications.find((a) => a.pid === application.pid) || application;
  const existing = s.config.rules.find(
    (r) => r.executable.toLowerCase() === application.executable.toLowerCase(),
  );
  const [mode, setMode] = useState<RoutingMode>(existing?.mode || "auto");
  const [endpoint, setEndpoint] = useState(existing?.endpointId || "");
  async function save() {
    if (
      await saveRule({
        id: existing?.id || crypto.randomUUID(),
        executable: application.executable,
        mode,
        endpointId: mode === "fixed" ? endpoint : null,
        enabled: true,
        priority: existing?.priority ?? 100,
        createdAt: "",
        updatedAt: "",
      })
    )
      onClose();
  }
  return (
    <Dialog
      open
      onOpenChange={(value) => !value && onClose()}
      title={application.name}
      description="Configure this executable's audio routing."
    >
      <div className="dialog-body">
        <dl className="details">
          <dt>Executable</dt>
          <dd className="break-all">{application.executable}</dd>
          <dt>Process</dt>
          <dd>{application.pid}</dd>
          <dt>Current monitor</dt>
          <dd>
            {s.monitors.find((m) => m.id === current.monitorId)?.name ||
              "No eligible window"}
          </dd>
          <dt>Observed outputs</dt>
          <dd>
            {current.endpointIds
              .map((id) => s.devices.find((d) => d.id === id)?.name || id)
              .join(", ")}
          </dd>
          <dt>Routing status</dt>
          <dd>{current.status}</dd>
          <dt>Last verified switch</dt>
          <dd>{time(current.lastSwitch)}</dd>
        </dl>
        {current.lastError && (
          <div className="error-message">{current.lastError}</div>
        )}
        <label className="field">
          Routing mode
          <select
            value={mode}
            onChange={(e) => setMode(e.target.value as RoutingMode)}
          >
            <option value="auto">Follow monitor</option>
            <option value="fixed">Fixed audio device</option>
            <option value="default">Windows default</option>
            <option value="ignore">Ignore this application</option>
          </select>
        </label>
        {mode === "fixed" && (
          <label className="field">
            Output device
            <DeviceSelect
              value={endpoint}
              onChange={setEndpoint}
              devices={s.devices}
            />
          </label>
        )}
        <BrowserNotice />
        <div className="dialog-actions">
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="default"
            disabled={busy || (mode === "fixed" && !endpoint)}
            onClick={() => void save()}
          >
            Save application rule
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
