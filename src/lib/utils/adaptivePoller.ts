export type AdaptivePollerOptions = {
  /** 执行一次轮询；返回 true 表示目标正处于活跃状态，应保持快速轮询。 */
  poll: () => Promise<boolean>;
  liveIntervalMs: number;
  idleIntervalMs: number;
};

/**
 * 活跃时快、空闲时慢的轮询器。
 *
 * 悬浮字幕窗一天里绝大部分时间是隐藏的。固定高频轮询会让常驻托盘的应用始终占着 CPU，
 * 所以隐藏时只保留低频心跳：事件正常送达时心跳什么都不用做，事件丢失时也能在一个心跳内恢复。
 */
export function createAdaptivePoller(options: AdaptivePollerOptions) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running = false;
  let inFlight = false;
  let live = false;
  let wakeRequested = false;

  function schedule(delayMs: number) {
    if (!running) return;
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => void tick(), delayMs);
  }

  async function tick() {
    timer = undefined;
    if (!running || inFlight) return;
    inFlight = true;
    wakeRequested = false;
    try {
      live = await options.poll();
    } catch {
      live = false;
    } finally {
      inFlight = false;
    }
    if (wakeRequested) {
      schedule(0);
      return;
    }
    schedule(live ? options.liveIntervalMs : options.idleIntervalMs);
  }

  return {
    start() {
      if (running) return;
      running = true;
      schedule(0);
    },
    stop() {
      running = false;
      if (timer !== undefined) clearTimeout(timer);
      timer = undefined;
    },
    /** 外部信号表明目标刚变为活跃：立即轮询一次，不等下一个心跳。 */
    wake() {
      if (!running || live) return;
      if (inFlight) {
        // 正在进行的这次轮询可能恰好在目标变活跃之前取的样，结束后要再取一次。
        wakeRequested = true;
        return;
      }
      schedule(0);
    },
    get live() {
      return live;
    },
  };
}
