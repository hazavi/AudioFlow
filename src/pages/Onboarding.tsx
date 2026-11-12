import { useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Monitor,
  Radio,
  Volume2,
} from "lucide-react";
import { settings, useSystem } from "../services/tauri";
import { Button } from "../components/ui/button";
import { Switch } from "../components/ui/switch";
import { Dialog } from "../components/ui/dialog";
import {
  BrowserNotice,
  MonitorArrangement,
  MonitorCard,
} from "../components/shared";
export function Onboarding() {
  const s = useSystem((state) => state.snapshot)!;
  const busy = useSystem((state) => state.busy);
  const [step, setStep] = useState(0);
  const [enable, setEnable] = useState(true);
  const [autostart, setAutostart] = useState(false);
  const [minimized, setMinimized] = useState(false);
  const titles = [
    "Welcome to AudioFlow",
    "Your connected displays",
    "Assign your audio outputs",
    "Enable automatic routing",
    "AudioFlow is ready",
  ];
  return (
    <Dialog
      open
      dismissible={false}
      onOpenChange={() => {}}
      title={titles[step]}
      description="Your sound. Your screen. Automatically."
    >
      <div className="onboarding dialog-body">
        <div className="setup-steps">
          {titles.map((_, i) => (
            <span key={i} className={i <= step ? "active" : ""} />
          ))}
        </div>
        {step === 0 && (
          <>
            <div className="welcome-icon">
              <Radio size={36} />
            </div>
            <h2>Make your audio follow your screens.</h2>
            <p>
              Assign a playback device to each monitor. Supported applications
              will use that output when their window moves.
            </p>
            <BrowserNotice />
          </>
        )}
        {step === 1 && (
          <>
            <MonitorArrangement monitors={s.monitors} compact />
            <p>
              {s.monitors.length === 1
                ? "One display is connected. You can still configure AudioFlow and add more displays later."
                : `${s.monitors.length} displays detected from Windows.`}
            </p>
            <p className="muted">
              Display assignments are remembered across restarts and
              reconnection.
            </p>
          </>
        )}
        {step === 2 && (
          <>
            <div className="setup-monitors">
              {s.monitors.map((m, i) => (
                <MonitorCard key={m.id} monitor={m} index={i} />
              ))}
            </div>
            <p className="muted">
              Test sounds play quietly through the selected endpoint. Unassigned
              displays are left alone.
            </p>
          </>
        )}
        {step === 3 && (
          <>
            <BrowserNotice />
            <div className="setup-choice">
              <Radio size={18} />
              <div>
                <strong>Enable automatic routing</strong>
                <p>Control applications without a manual Windows output.</p>
              </div>
              <Switch
                aria-label="Enable automatic routing in setup"
                checked={enable}
                disabled={!s.routingSupported}
                onCheckedChange={setEnable}
              />
            </div>
            <div className="setup-choice">
              <Monitor size={18} />
              <strong>Launch with Windows</strong>
              <Switch
                aria-label="Launch with Windows in setup"
                checked={autostart}
                onCheckedChange={setAutostart}
              />
            </div>
            <div className="setup-choice">
              <Volume2 size={18} />
              <strong>Start minimized to tray</strong>
              <Switch
                aria-label="Start minimized in setup"
                checked={minimized}
                onCheckedChange={setMinimized}
              />
            </div>
            {!s.routingSupported && (
              <p className="warning">{s.compatibility}</p>
            )}
          </>
        )}
        {step === 4 && (
          <>
            <div className="welcome-icon">
              <Check size={36} />
            </div>
            <h2>Your monitor setup is saved.</h2>
            <p>
              {s.config.settings.routingEnabled
                ? "Supported applications will now follow their monitor’s assigned audio output."
                : "Automatic routing is paused. Enable it from the dashboard when you’re ready."}
            </p>
          </>
        )}
        <div className="dialog-actions">
          {step > 0 && step < 4 && (
            <Button onClick={() => setStep(step - 1)}>
              <ArrowLeft size={14} /> Back
            </Button>
          )}
          <Button
            variant="default"
            disabled={busy}
            onClick={async () => {
              if (step === 3) {
                if (
                  await settings({
                    routingEnabled: enable && s.routingSupported,
                    launchOnStartup: autostart,
                    startMinimized: minimized,
                  })
                )
                  setStep(4);
              } else if (step === 4)
                await settings({ onboardingCompleted: true });
              else setStep(step + 1);
            }}
          >
            {step === 0
              ? "Get started"
              : step === 4
                ? "Open dashboard"
                : "Continue"}{" "}
            <ArrowRight size={14} />
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
