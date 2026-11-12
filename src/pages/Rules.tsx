import { useState } from "react";
import { Pencil, Plus, Search, Trash2 } from "lucide-react";
import { action, saveRule, useSystem } from "../services/tauri";
import { Button } from "../components/ui/button";
import { Switch } from "../components/ui/switch";
import { Dialog } from "../components/ui/dialog";
import { DeviceSelect, Empty, Section } from "../components/shared";
import type { Rule, RoutingMode } from "../types";
export function Rules() {
  const s = useSystem((state) => state.snapshot)!;
  const busy = useSystem((state) => state.busy);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Rule | null>(null);
  const [deleting, setDeleting] = useState<Rule | null>(null);
  const rules = s.config.rules
    .filter((r) => r.executable.toLowerCase().includes(query.toLowerCase()))
    .sort((a, b) => b.priority - a.priority || a.id.localeCompare(b.id));
  function add() {
    setEditing({
      id: crypto.randomUUID(),
      executable: "",
      mode: "fixed",
      endpointId: null,
      enabled: true,
      priority: 100,
      createdAt: "",
      updatedAt: "",
    });
  }
  return (
    <>
      <div className="toolbar">
        <div className="search">
          <Search size={15} />
          <input
            placeholder="Search routing rules…"
            aria-label="Search rules"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <Button variant="default" onClick={add}>
          <Plus size={14} /> Add rule
        </Button>
      </div>
      <Section title="Application rules" meta={`${rules.length} rules`}>
        {!rules.length ? (
          <Empty
            title="No matching rules"
            detail="Add an exception to keep an application on a fixed output, follow its monitor, or leave it alone."
          />
        ) : (
          rules.map((r) => (
            <div className="rule-row" key={r.id}>
              <Switch
                aria-label={`Enable rule for ${r.executable}`}
                checked={r.enabled}
                disabled={busy}
                onCheckedChange={(enabled) => void saveRule({ ...r, enabled })}
              />
              <div className="grow truncate">
                <strong>{r.executable}</strong>
                <small>
                  {r.mode === "fixed"
                    ? s.devices.find((d) => d.id === r.endpointId)?.name ||
                      "Output disconnected"
                    : r.mode === "auto"
                      ? "Follow monitor"
                      : r.mode === "default"
                        ? "Windows default"
                        : "Leave audio settings alone"}
                </small>
              </div>
              <span className="label">Priority {r.priority}</span>
              <Button
                size="icon"
                variant="ghost"
                aria-label="Edit rule"
                onClick={() => setEditing(r)}
              >
                <Pencil size={14} />
              </Button>
              <Button
                size="icon"
                variant="ghost"
                aria-label="Delete rule"
                onClick={() => setDeleting(r)}
              >
                <Trash2 size={14} />
              </Button>
            </div>
          ))
        )}
      </Section>
      <div className="info-note">
        Higher priority rules run first. Ties prefer fixed outputs, then ignore,
        follow monitor, and Windows default. Executable names match exactly; a
        full path restricts a rule to that installation.
      </div>
      {editing && (
        <RuleEditor
          key={editing.id}
          rule={editing}
          onClose={() => setEditing(null)}
        />
      )}
      <Dialog
        open={!!deleting}
        onOpenChange={(v) => !v && setDeleting(null)}
        title="Delete routing rule?"
        description="Automatic monitor routing will apply again if no other rule matches."
      >
        <div className="dialog-body">
          <p className="break-all">{deleting?.executable}</p>
          <div className="dialog-actions">
            <Button onClick={() => setDeleting(null)}>Cancel</Button>
            <Button
              variant="destructive"
              disabled={busy}
              onClick={async () => {
                if (
                  deleting &&
                  (await action("delete_rule", { id: deleting.id }))
                )
                  setDeleting(null);
              }}
            >
              Delete rule
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
function RuleEditor({ rule, onClose }: { rule: Rule; onClose: () => void }) {
  const devices = useSystem((state) => state.snapshot!.devices);
  const busy = useSystem((state) => state.busy);
  const [draft, setDraft] = useState(rule);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={rule.createdAt ? "Edit routing rule" : "Add routing rule"}
      description="Rules take priority over automatic monitor assignments."
    >
      <form
        className="dialog-body"
        onSubmit={async (e) => {
          e.preventDefault();
          if (await saveRule(draft)) onClose();
        }}
      >
        <label className="field">
          Executable name or full path
          <input
            required
            autoFocus
            placeholder="For example: Discord.exe"
            value={draft.executable}
            onChange={(e) => setDraft({ ...draft, executable: e.target.value })}
          />
        </label>
        <label className="field">
          Routing mode
          <select
            value={draft.mode}
            onChange={(e) =>
              setDraft({ ...draft, mode: e.target.value as RoutingMode })
            }
          >
            <option value="fixed">Always use a device</option>
            <option value="auto">Follow monitor</option>
            <option value="default">Windows default</option>
            <option value="ignore">Ignore application</option>
          </select>
        </label>
        {draft.mode === "fixed" && (
          <label className="field">
            Output device
            <DeviceSelect
              value={draft.endpointId || ""}
              onChange={(endpointId) => setDraft({ ...draft, endpointId })}
              devices={devices}
            />
          </label>
        )}
        <label className="field">
          Priority
          <input
            type="number"
            min={-10000}
            max={10000}
            required
            value={draft.priority}
            onChange={(e) =>
              setDraft({ ...draft, priority: Number(e.target.value) })
            }
          />
        </label>
        <div className="dialog-actions">
          <Button type="button" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="default"
            disabled={
              busy ||
              !draft.executable.trim() ||
              (draft.mode === "fixed" && !draft.endpointId)
            }
          >
            Save rule
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
