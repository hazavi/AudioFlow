import { useState } from "react";
import { ArrowDownToLine, X } from "lucide-react";
import { useUpdateStore } from "../../stores/useUpdateStore";
import {
  checkUpdates,
  downloadUpdate,
  installUpdate,
} from "../../services/updater";
import { useSystem } from "../../services/tauri";
import { Button } from "../ui/button";
import { Dialog } from "../ui/dialog";
import { UpdateProgress } from "./UpdateProgress";
import { UpdateStatus } from "./UpdateStatus";
export function UpdateBanner() {
  const update = useUpdateStore();
  const snapshot = useSystem((s) => s.snapshot);
  const [confirm, setConfirm] = useState(false);
  const [notes, setNotes] = useState(false);
  if (
    !update.version ||
    update.dismissed ||
    Date.now() < update.laterUntil ||
    ![
      "available",
      "downloading",
      "ready",
      "installing",
      "restarting",
      "failed",
    ].includes(update.status)
  )
    return null;
  if (
    !snapshot?.config.settings.updateNotifications &&
    !update.manualCheck &&
    update.status === "available"
  )
    return null;
  const locked = ["downloading", "installing", "restarting"].includes(
    update.status,
  );
  return (
    <>
      <div className="update-banner">
        <ArrowDownToLine size={19} className="accent" />
        <div className="grow">
          <div className="inline">
            <span className="badge accent">UPDATE</span>
            <strong>AudioFlow v{update.version}</strong>
            <span className="muted small">Current v{snapshot?.version}</span>
          </div>
          <UpdateStatus />
          {update.status === "downloading" && <UpdateProgress />}
          {update.error && <p className="danger small">{update.error}</p>}
        </div>
        {update.status === "available" && (
          <Button
            variant="default"
            size="sm"
            onClick={() => void downloadUpdate()}
          >
            Update now
          </Button>
        )}
        {update.status === "ready" && (
          <Button variant="default" size="sm" onClick={() => setConfirm(true)}>
            Install & restart
          </Button>
        )}
        {update.status === "failed" && (
          <Button size="sm" onClick={() => void checkUpdates(true)}>
            Retry
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={() => setNotes(true)}>
          Release notes
        </Button>
        {!locked && (
          <>
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                useUpdateStore.setState({
                  laterUntil: Date.now() + 60 * 60 * 1000,
                })
              }
            >
              Later
            </Button>
            <Button
              size="icon"
              variant="ghost"
              aria-label="Dismiss update"
              onClick={() => useUpdateStore.setState({ dismissed: true })}
            >
              <X size={14} />
            </Button>
          </>
        )}
      </div>
      <Dialog
        open={confirm}
        onOpenChange={setConfirm}
        title={`Install AudioFlow v${update.version}?`}
        description="Installing closes and restarts AudioFlow on Windows. Audio routing pauses during installation; your settings and assignments are preserved."
      >
        <div className="dialog-body">
          <p>
            The update's cryptographic signature is verified by Tauri.
            Installation only continues when verification succeeds.
          </p>
          <div className="dialog-actions">
            <Button onClick={() => setConfirm(false)}>Later</Button>
            <Button
              variant="default"
              onClick={() => {
                setConfirm(false);
                void installUpdate();
              }}
            >
              Install & restart
            </Button>
          </div>
        </div>
      </Dialog>
      <Dialog
        open={notes}
        onOpenChange={setNotes}
        title={`AudioFlow v${update.version}`}
        description="Release notes from the signed update source."
      >
        <div className="dialog-body">
          <pre className="release-notes">
            {update.notes || "No release notes were provided."}
          </pre>
        </div>
      </Dialog>
    </>
  );
}
