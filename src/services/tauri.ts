import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { useUI } from "../stores/ui";
import type { Snapshot, Settings, Rule, Page } from "../types";
export const desktop = isTauri();
interface State {
  snapshot: Snapshot | null;
  loading: boolean;
  busy: boolean;
  notice: string | null;
  error: string | null;
}
export const useSystem = create<State>(() => ({
  snapshot: null,
  loading: true,
  busy: false,
  notice: null,
  error: null,
}));
const receive = (snapshot: Snapshot) => {
  useSystem.setState({ snapshot, loading: false });
  if (useUI.getState().theme !== snapshot.config.settings.theme) {
    useUI.getState().setTheme(snapshot.config.settings.theme);
  }
};
export async function connect() {
  if (!desktop) {
    useSystem.setState({
      loading: false,
      error:
        "Native Windows services are unavailable in a browser. Run npm run desktop to connect to real monitors and audio sessions.",
    });
    return () => {};
  }
  const unlisten = await listen<Snapshot>("state-changed", (e) =>
    receive(e.payload),
  );
  const navigation = await listen<Page>("navigate", (e) =>
    useUI.getState().setPage(e.payload),
  );
  try {
    receive(await invoke<Snapshot>("get_snapshot"));
  } catch (e) {
    useSystem.setState({ loading: false, error: String(e) });
  }
  return () => {
    unlisten();
    navigation();
  };
}
export async function action(
  command: string,
  args?: Record<string, unknown>,
  success?: string,
) {
  if (!desktop) {
    useSystem.setState({
      error:
        "Open the Tauri desktop application to use Windows audio controls.",
    });
    return false;
  }
  useSystem.setState({ busy: true, error: null, notice: null });
  try {
    await invoke(command, args);
    receive(await invoke<Snapshot>("get_snapshot"));
    if (success) useSystem.setState({ notice: success });
    return true;
  } catch (e) {
    useSystem.setState({ error: String(e) });
    return false;
  } finally {
    useSystem.setState({ busy: false });
  }
}
export function settings(patch: Partial<Settings>) {
  const current = useSystem.getState().snapshot?.config.settings;
  return current
    ? action("save_settings", { settings: { ...current, ...patch } })
    : Promise.resolve(false);
}
export function saveRule(rule: Rule) {
  return action("save_rule", { rule }, "Application rule saved.");
}
export async function exportText(command: "export_config" | "export_activity") {
  return invoke<string>(command);
}
