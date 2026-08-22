import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as ipc from "../api/ipc";
import {
  scheduleReloadSequence,
  whenReloadsSettled,
  resetLiveReloadForTests,
} from "./liveReload";

vi.mock("../api/ipc");

/** A reloadSequence mock whose completion the test controls. */
function deferredReload() {
  const resolvers: Array<() => void> = [];
  vi.mocked(ipc.reloadSequence).mockImplementation(
    () => new Promise<void>((resolve) => resolvers.push(resolve)),
  );
  return {
    resolvers,
    /** Complete the oldest outstanding call and let its `.then` chain run. */
    async settleOne() {
      const next = resolvers.shift();
      expect(next, "no outstanding reloadSequence to settle").toBeDefined();
      next!();
      // Two microtask turns: the catch link, then the then link.
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    },
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  resetLiveReloadForTests();
});

afterEach(() => {
  resetLiveReloadForTests();
  vi.restoreAllMocks();
});

describe("scheduleReloadSequence", () => {
  it("issues the first reload immediately so a gesture feels connected", () => {
    vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
    scheduleReloadSequence();
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(1);
  });

  it("collapses a burst during one round trip into a single trailing reload", async () => {
    const d = deferredReload();

    // One drag: 20 input events before the first reload comes back. Without
    // coalescing this is 20 snapshot rebuilds.
    for (let i = 0; i < 20; i++) scheduleReloadSequence();
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(1);

    await d.settleOne();
    // Exactly one trailing reload for the whole burst — not 19.
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);

    await d.settleOne();
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);
  });

  it("always issues a trailing reload, so the final value is never dropped", async () => {
    const d = deferredReload();

    scheduleReloadSequence(); // leading, in flight
    scheduleReloadSequence(); // the value the user actually released on
    await d.settleOne();

    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);
  });

  it("does not coalesce calls that are already settled", async () => {
    vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);

    scheduleReloadSequence();
    await whenReloadsSettled();
    scheduleReloadSequence();
    await whenReloadsSettled();

    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);
  });

  it("a rejected reload does not wedge the coalescer", async () => {
    const err = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(ipc.reloadSequence).mockRejectedValueOnce(new Error("backend gone"));

    scheduleReloadSequence();
    await whenReloadsSettled();

    vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
    scheduleReloadSequence();
    await whenReloadsSettled();

    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);
    expect(err).toHaveBeenCalled();
  });

  it("whenReloadsSettled waits for the trailing reload too", async () => {
    let outstanding = 0;
    vi.mocked(ipc.reloadSequence).mockImplementation(async () => {
      outstanding++;
      await Promise.resolve();
      outstanding--;
    });

    scheduleReloadSequence();
    scheduleReloadSequence();
    await whenReloadsSettled();

    expect(outstanding).toBe(0);
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);
  });
});
