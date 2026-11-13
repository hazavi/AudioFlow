// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({
  check: vi.fn(),
  invoke: vi.fn(),
  relaunch: vi.fn(),
  ready: true,
  subscribers: new Set<() => void>(),
  tick: null as (() => void) | null,
  snapshot: {
    updaterConfigured: true,
    config: { settings: { automaticUpdates: true } },
  },
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event: string, callback: () => void) => {
    mocks.tick = callback;
    return () => {
      mocks.tick = null;
    };
  }),
}));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: mocks.relaunch }));
vi.mock("./tauri", () => ({
  desktop: true,
  useSystem: {
    getState: () => ({ snapshot: mocks.ready ? mocks.snapshot : null }),
    subscribe: (callback: () => void) => {
      mocks.subscribers.add(callback);
      return () => mocks.subscribers.delete(callback);
    },
  },
}));
const cleanups: (() => void)[] = [];
beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  mocks.snapshot.updaterConfigured = true;
  mocks.snapshot.config.settings.automaticUpdates = true;
  mocks.ready = true;
  mocks.tick = null;
  mocks.subscribers.clear();
  mocks.check.mockReset().mockResolvedValue(null);
  mocks.invoke.mockResolvedValue(undefined);
});
afterEach(() => {
  cleanups.splice(0).forEach((stop) => stop());
  vi.useRealTimers();
});
describe("signed updater lifecycle", () => {
  it("does not request an invented endpoint for an unconfigured build", async () => {
    mocks.snapshot.updaterConfigured = false;
    const service = await import("./updater");
    await service.checkUpdates(true);
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(mocks.check).not.toHaveBeenCalled();
    expect(useUpdateStore.getState().status).toBe("unconfigured");
  });
  it("respects disabled automatic checks while allowing manual checks", async () => {
    mocks.snapshot.config.settings.automaticUpdates = false;
    mocks.check.mockResolvedValue(null);
    const service = await import("./updater");
    await service.checkUpdates();
    expect(mocks.check).not.toHaveBeenCalled();
    await service.checkUpdates(true);
    expect(mocks.check).toHaveBeenCalledTimes(1);
  });
  it("serializes overlapping checks", async () => {
    let finish: (value: null) => void = () => {};
    mocks.check.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const service = await import("./updater");
    const first = service.checkUpdates(true);
    await service.checkUpdates(true);
    expect(mocks.check).toHaveBeenCalledTimes(1);
    finish(null);
    await first;
  });
  it("uses real byte events and never installs after download alone", async () => {
    const install = vi.fn();
    const close = vi.fn();
    mocks.check.mockResolvedValue({
      version: "0.2.0",
      body: "Release notes",
      close,
      install,
      download: async (event: (e: unknown) => void) => {
        event({ event: "Started", data: {} });
        event({ event: "Progress", data: { chunkLength: 4096 } });
        event({ event: "Finished" });
      },
    });
    const service = await import("./updater");
    await service.checkUpdates(true);
    await service.downloadUpdate();
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState()).toMatchObject({
      downloaded: 4096,
      total: null,
      status: "ready",
    });
    expect(install).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
  it("restores the audio worker if installation fails", async () => {
    mocks.check.mockResolvedValue({
      version: "0.2.0",
      close: vi.fn(),
      download: vi.fn().mockResolvedValue(undefined),
      install: vi.fn().mockRejectedValue(new Error("Signature rejected")),
    });
    const service = await import("./updater");
    await service.checkUpdates(true);
    await service.downloadUpdate();
    await service.installUpdate();
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("failed");
    expect(mocks.relaunch).not.toHaveBeenCalled();
    expect(mocks.invoke.mock.calls.map((call) => call[0])).toEqual([
      "prepare_update",
      "resume_after_update",
    ]);
  });
  it("downloads, prepares the worker, and installs after a single confirmation", async () => {
    const order: string[] = [];
    mocks.invoke.mockImplementation(async (command: string) => {
      order.push(command);
    });
    mocks.relaunch.mockImplementation(async () => {
      order.push("relaunch");
    });
    mocks.check.mockResolvedValue({
      version: "0.2.0",
      close: vi.fn(),
      download: vi.fn(async () => {
        order.push("download");
      }),
      install: vi.fn(async () => {
        order.push("install");
      }),
    });
    const service = await import("./updater");
    await service.checkUpdates(true);
    await service.applyUpdate();
    expect(order).toEqual([
      "download",
      "prepare_update",
      "install",
      "relaunch",
    ]);
  });
  it("keeps routing running when a download or signature check fails", async () => {
    const install = vi.fn();
    mocks.check.mockResolvedValue({
      version: "0.2.0",
      close: vi.fn(),
      install,
      download: vi.fn().mockRejectedValue(new Error("Signature rejected")),
    });
    const service = await import("./updater");
    await service.checkUpdates(true);
    await service.applyUpdate();
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("failed");
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(install).not.toHaveBeenCalled();
  });
  it("does not mark an offline check as up to date", async () => {
    mocks.check.mockRejectedValue(new Error("Offline"));
    const service = await import("./updater");
    await service.checkUpdates();
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("failed");
    expect(useUpdateStore.getState().error).toContain("Offline");
  });
  it("waits for a slow initial snapshot and detects an update without a click", async () => {
    vi.useFakeTimers();
    mocks.ready = false;
    mocks.check.mockResolvedValue({
      version: "0.2.0",
      body: "New release",
      close: vi.fn(),
    });
    const service = await import("./updater");
    cleanups.push(await service.startUpdater());
    await vi.advanceTimersByTimeAsync(120000);
    expect(mocks.check).not.toHaveBeenCalled();
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("idle");
    mocks.ready = true;
    mocks.subscribers.forEach((callback) => callback());
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.check).toHaveBeenCalledTimes(1);
    expect(useUpdateStore.getState()).toMatchObject({
      status: "available",
      version: "0.2.0",
      manualCheck: false,
    });
  });
  it("detects a release published after startup on its recurring timer", async () => {
    vi.useFakeTimers();
    const service = await import("./updater");
    cleanups.push(await service.startUpdater());
    await vi.advanceTimersByTimeAsync(4 * 60000);
    expect(mocks.check).toHaveBeenCalledTimes(1);
    mocks.check.mockResolvedValue({ version: "0.2.0", close: vi.fn() });
    await vi.advanceTimersByTimeAsync(60000);
    expect(mocks.check).toHaveBeenCalledTimes(2);
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("available");
  });
  it("checks from native ticks while frontend timers are suspended and cleans up", async () => {
    vi.useFakeTimers();
    const service = await import("./updater");
    const stop = await service.startUpdater();
    cleanups.push(stop);
    expect(mocks.check).toHaveBeenCalledTimes(1);
    vi.setSystemTime(Date.now() + 5 * 60000);
    mocks.tick?.();
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.check).toHaveBeenCalledTimes(2);
    stop();
    expect(mocks.subscribers.size).toBe(0);
    expect(mocks.tick).toBeNull();
    await vi.advanceTimersByTimeAsync(10 * 60000);
    window.dispatchEvent(new Event("focus"));
    expect(mocks.check).toHaveBeenCalledTimes(2);
  });
  it("retries offline failures automatically after a minute", async () => {
    vi.useFakeTimers();
    mocks.check.mockRejectedValueOnce(new Error("Offline"));
    const service = await import("./updater");
    cleanups.push(await service.startUpdater());
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("failed");
    await vi.advanceTimersByTimeAsync(60000);
    expect(mocks.check).toHaveBeenCalledTimes(2);
    expect(useUpdateStore.getState().status).toBe("upToDate");
  });
  it("honors disabled checks and starts when automatic checks are enabled", async () => {
    vi.useFakeTimers();
    mocks.snapshot.config.settings.automaticUpdates = false;
    const service = await import("./updater");
    cleanups.push(await service.startUpdater());
    await vi.advanceTimersByTimeAsync(5 * 60000);
    expect(mocks.check).not.toHaveBeenCalled();
    mocks.snapshot.config.settings.automaticUpdates = true;
    mocks.subscribers.forEach((callback) => callback());
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.check).toHaveBeenCalledTimes(1);
  });
});
