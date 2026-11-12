import { beforeEach, describe, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({
  check: vi.fn(),
  invoke: vi.fn(),
  relaunch: vi.fn(),
  snapshot: {
    updaterConfigured: true,
    config: { settings: { automaticUpdates: true } },
  },
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: mocks.relaunch }));
vi.mock("./tauri", () => ({
  desktop: true,
  useSystem: { getState: () => ({ snapshot: mocks.snapshot }) },
}));
beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  mocks.snapshot.updaterConfigured = true;
  mocks.snapshot.config.settings.automaticUpdates = true;
  mocks.invoke.mockResolvedValue(undefined);
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
  it("does not mark an offline check as up to date", async () => {
    mocks.check.mockRejectedValue(new Error("Offline"));
    const service = await import("./updater");
    await service.checkUpdates();
    const { useUpdateStore } = await import("../stores/useUpdateStore");
    expect(useUpdateStore.getState().status).toBe("failed");
    expect(useUpdateStore.getState().error).toContain("Offline");
  });
});
