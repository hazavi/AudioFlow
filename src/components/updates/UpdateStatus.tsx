import { useUpdateStore } from "../../stores/useUpdateStore";
export const statusNames = {
  idle: "Not checked yet",
  checking: "Checking for updates…",
  available: "Update available",
  downloading: "Downloading update",
  ready: "Ready to install",
  installing: "Verifying / installing",
  restarting: "Restarting AudioFlow",
  upToDate: "Up to date",
  failed: "Update failed",
  unconfigured: "Release source not configured",
};
export function UpdateStatus() {
  const status = useUpdateStore((s) => s.status);
  return <span className="label">{statusNames[status]}</span>;
}
