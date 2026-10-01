import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createAdaptivePoller } from "$lib/utils/adaptivePoller";

const liveIntervalMs = 250;
const idleIntervalMs = 2000;

function createPoller(isLive: () => boolean) {
  const poll = vi.fn(async () => isLive());
  const poller = createAdaptivePoller({ poll, liveIntervalMs, idleIntervalMs });
  return { poll, poller };
}

describe("adaptive poller", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps only a slow heartbeat while the target is idle", async () => {
    const { poll, poller } = createPoller(() => false);
    poller.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(poll).toHaveBeenCalledTimes(1);

    // 回归：此前不管字幕窗是否可见都固定 250ms 轮询一次，10 秒要问 40 次。
    await vi.advanceTimersByTimeAsync(10_000);
    expect(poll).toHaveBeenCalledTimes(6);
    expect(poller.live).toBe(false);
    poller.stop();
  });

  it("polls at the live interval while the target reports active", async () => {
    const { poll, poller } = createPoller(() => true);
    poller.start();
    await vi.advanceTimersByTimeAsync(0);

    await vi.advanceTimersByTimeAsync(1_000);
    expect(poll).toHaveBeenCalledTimes(5);
    expect(poller.live).toBe(true);
    poller.stop();
  });

  it("wakes immediately instead of waiting for the next heartbeat", async () => {
    let live = false;
    const { poll, poller } = createPoller(() => live);
    poller.start();
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(300);
    expect(poll).toHaveBeenCalledTimes(1);

    live = true;
    poller.wake();
    await vi.advanceTimersByTimeAsync(0);
    expect(poll).toHaveBeenCalledTimes(2);
    expect(poller.live).toBe(true);

    await vi.advanceTimersByTimeAsync(liveIntervalMs);
    expect(poll).toHaveBeenCalledTimes(3);
    poller.stop();
  });

  it("drops back to the heartbeat once the target goes idle again", async () => {
    let live = true;
    const { poll, poller } = createPoller(() => live);
    poller.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(poller.live).toBe(true);

    live = false;
    await vi.advanceTimersByTimeAsync(liveIntervalMs);
    expect(poller.live).toBe(false);
    const callsWhenIdle = poll.mock.calls.length;

    await vi.advanceTimersByTimeAsync(idleIntervalMs - 1);
    expect(poll).toHaveBeenCalledTimes(callsWhenIdle);
    await vi.advanceTimersByTimeAsync(1);
    expect(poll).toHaveBeenCalledTimes(callsWhenIdle + 1);
    poller.stop();
  });

  it("re-polls after a wake that arrives while a poll is still in flight", async () => {
    let live = false;
    let release: (() => void) | undefined;
    const poll = vi.fn(async () => {
      const sampled = live;
      await new Promise<void>((resolve) => {
        release = resolve;
      });
      return sampled;
    });
    const poller = createAdaptivePoller({ poll, liveIntervalMs, idleIntervalMs });
    poller.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(poll).toHaveBeenCalledTimes(1);

    // 这次轮询在目标变活跃之前就取了样；结束后必须马上再问一次，而不是等 2 秒心跳。
    live = true;
    poller.wake();
    release?.();
    await vi.advanceTimersByTimeAsync(0);
    expect(poll).toHaveBeenCalledTimes(2);
    release?.();
    await vi.advanceTimersByTimeAsync(0);
    expect(poller.live).toBe(true);
    poller.stop();
  });

  it("treats a failed poll as idle and keeps running", async () => {
    let fail = true;
    const poll = vi.fn(async () => {
      if (fail) throw new Error("ipc unavailable");
      return true;
    });
    const poller = createAdaptivePoller({ poll, liveIntervalMs, idleIntervalMs });
    poller.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(poller.live).toBe(false);

    fail = false;
    await vi.advanceTimersByTimeAsync(idleIntervalMs);
    expect(poll).toHaveBeenCalledTimes(2);
    expect(poller.live).toBe(true);
    poller.stop();
  });

  it("stops polling after stop", async () => {
    const { poll, poller } = createPoller(() => true);
    poller.start();
    await vi.advanceTimersByTimeAsync(0);
    poller.stop();

    await vi.advanceTimersByTimeAsync(5_000);
    expect(poll).toHaveBeenCalledTimes(1);
  });
});
