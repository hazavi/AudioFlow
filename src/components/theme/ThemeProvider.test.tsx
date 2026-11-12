// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render } from "@testing-library/react";
import { ThemeProvider } from "./ThemeProvider";
import { useUI } from "../../stores/ui";
let dark = false;
let notify: (() => void) | undefined;
beforeEach(() => {
  localStorage.clear();
  dark = false;
  notify = undefined;
  vi.stubGlobal("matchMedia", () => ({
    get matches() {
      return dark;
    },
    addEventListener: (_: string, fn: () => void) => {
      notify = fn;
    },
    removeEventListener: () => {
      notify = undefined;
    },
  }));
  useUI.setState({ theme: "dark" });
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
describe("desktop theme provider", () => {
  it("switches in both directions without remounting the application", () => {
    render(
      <ThemeProvider>
        <span>AudioFlow</span>
      </ThemeProvider>,
    );
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    act(() => useUI.getState().setTheme("light"));
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.style.colorScheme).toBe("light");
    act(() => useUI.getState().setTheme("dark"));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
  it("responds to Windows changes only in system mode", () => {
    useUI.setState({ theme: "system" });
    render(
      <ThemeProvider>
        <span>AudioFlow</span>
      </ThemeProvider>,
    );
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    act(() => {
      dark = true;
      notify?.();
    });
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    act(() => useUI.getState().setTheme("light"));
    act(() => {
      dark = true;
      notify?.();
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
  it("persists the preference for startup before React mounts", () => {
    act(() => useUI.getState().setTheme("system"));
    expect(JSON.parse(localStorage.getItem("audioflow-ui")!).state.theme).toBe(
      "system",
    );
  });
});
