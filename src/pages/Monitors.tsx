import { useSystem } from "../services/tauri";
import { MonitorArrangement, MonitorCard, Section } from "../components/shared";
export function Monitors() {
  const s = useSystem((state) => state.snapshot)!;
  const disconnected = Object.keys(s.config.assignments).filter(
    (id) => !s.monitors.some((m) => m.id === id),
  );
  return (
    <>
      <Section title="Desktop arrangement" meta="From Windows">
        <MonitorArrangement monitors={s.monitors} />
      </Section>
      <div className="monitor-grid standalone">
        {s.monitors.map((m, i) => (
          <MonitorCard key={m.id} monitor={m} index={i} details />
        ))}
      </div>
      {disconnected.length > 0 && (
        <div className="info-note">
          Assignments for {disconnected.length} disconnected display(s) are
          preserved. They will apply when the same display reconnects.
        </div>
      )}
      <div className="info-note">
        Displays and audio devices are matched only through your assignments.
        Windows display order is never used to guess a speaker.
      </div>
    </>
  );
}
