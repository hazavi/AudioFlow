import { create } from "zustand";
export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "restarting"
  | "upToDate"
  | "failed"
  | "unconfigured";
export const useUpdateStore = create<{
  status: UpdateStatus;
  version: string | null;
  notes: string;
  lastChecked: string | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  dismissed: boolean;
  laterUntil: number;
  manualCheck: boolean;
}>(() => ({
  status: "idle",
  version: null,
  notes: "",
  lastChecked: null,
  downloaded: 0,
  total: null,
  error: null,
  dismissed: false,
  laterUntil: 0,
  manualCheck: false,
}));
