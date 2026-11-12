import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Page, Theme } from "../types";
export const useUI = create(
  persist<{
    theme: Theme;
    page: Page;
    setTheme: (theme: Theme) => void;
    setPage: (page: Page) => void;
  }>(
    (set) => ({
      theme: "dark",
      page: "Dashboard",
      setTheme: (theme) => set({ theme }),
      setPage: (page) => set({ page }),
    }),
    {
      name: "audioflow-ui",
    },
  ),
);
