<script lang="ts">
  import type { OverlayMode, SessionPhase } from "$lib/types/app";

  type Props = {
    meterBars: number[];
    displayLines: string[];
    phase: SessionPhase;
    visible: boolean;
    dismissing: boolean;
    mode: OverlayMode;
    fontSize: number;
    rootStyle: string;
    meterBarScale: (bar: number) => number;
    textElement?: HTMLDivElement | null;
  };

  let {
    meterBars,
    displayLines,
    phase,
    visible,
    dismissing,
    mode,
    fontSize,
    rootStyle,
    meterBarScale,
    textElement = $bindable<HTMLDivElement | null>(null),
  }: Props = $props();
</script>

<main class="overlay-root" style={rootStyle}>
  <div class="overlay-caption" class:visible class:dismissing data-phase={phase}>
    <div class="overlay-voice-meter" aria-hidden="true">
      {#each meterBars as bar}
        <i style:--bar-scale={meterBarScale(bar)}></i>
      {/each}
    </div>
    <div
      class:single={mode === "single"}
      class:double={mode === "double"}
      class="overlay-caption-text"
      style={`font-size: ${fontSize}px`}
      bind:this={textElement}
    >
      {#key displayLines}
        {#each displayLines as line}
          <span>{line || "\u00a0"}</span>
        {/each}
      {/key}
    </div>
  </div>
</main>

<style>
  .overlay-root {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100vw;
    height: 100vh;
    padding: 2px 4px;
    overflow: hidden;
    background: transparent;
  }

  .overlay-caption {
    position: relative;
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    align-items: center;
    justify-content: flex-start;
    column-gap: 12px;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    padding: 6px 14px;
    overflow: hidden;
    color: var(--overlay-text, #ffffff);
    background: rgba(var(--overlay-bg-rgb, 23, 110, 230), var(--overlay-opacity, 0.9));
    border: 0;
    border-radius: 14px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.18);
    opacity: 0;
    font-family: "Microsoft YaHei", "Segoe UI", "PingFang SC", "SF Pro Display", "Noto Sans CJK SC", sans-serif;
    text-align: left;
  }

  .overlay-caption.visible {
    opacity: 1;
    animation: caption-in 100ms ease-out both;
  }

  .overlay-caption.dismissing {
    animation: caption-out 140ms ease-in both;
  }

  .overlay-caption.dismissing::after {
    position: absolute;
    inset: 0;
    border: 1px solid rgba(255, 255, 255, 0.72);
    border-radius: inherit;
    content: "";
    pointer-events: none;
    animation: caption-glint 140ms ease-out both;
  }

  .overlay-voice-meter {
    position: relative;
    display: inline-flex;
    align-items: center;
    align-self: center;
    justify-content: center;
    justify-self: center;
    gap: 2px;
    width: 31px;
    height: 25px;
    color: var(--overlay-text, #ffffff);
    background: color-mix(in srgb, currentColor 11%, transparent);
    border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    border-radius: 999px;
    opacity: 0.72;
    pointer-events: none;
    overflow: hidden;
    transition: opacity 85ms ease;
  }

  [data-phase="recording"] .overlay-voice-meter,
  [data-phase="stopping"] .overlay-voice-meter {
    opacity: 0.88;
  }

  .overlay-voice-meter i {
    width: 2px;
    height: 17px;
    background: currentColor;
    border-radius: 999px;
    transform: scaleY(var(--bar-scale));
    transition: transform 85ms ease-out;
  }

  [data-phase="waiting_final_result"] .overlay-voice-meter i {
    animation: meter-wait 820ms ease-in-out infinite alternate;
  }

  [data-phase="post_editing"] .overlay-voice-meter::before {
    position: absolute;
    inset: 0;
    background: linear-gradient(100deg, transparent 15%, currentColor, transparent 85%);
    content: "";
    animation: meter-polish 1050ms ease-in-out infinite;
  }

  .overlay-caption-text {
    display: grid;
    align-content: start;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    height: 100%;
    max-height: 100%;
    padding: 0 6px 0 0;
    overflow: hidden;
    color: inherit;
    box-sizing: border-box;
    font-weight: 400;
    line-height: 1.18;
    text-shadow: none;
    white-space: normal;
    overflow-wrap: normal;
  }

  .overlay-caption-text.single,
  .overlay-caption-text.double {
    align-content: center;
    text-align: left;
  }

  .overlay-caption-text span {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: clip;
    white-space: pre;
    animation: caption-text-in 80ms ease-out;
  }

  @keyframes caption-in {
    from { opacity: 0.68; transform: scale(0.985); }
    to { opacity: 1; transform: scale(1); }
  }

  @keyframes caption-out {
    from { opacity: 1; transform: scale(1); }
    to { opacity: 0; transform: scale(0.985); }
  }

  @keyframes caption-glint {
    0% { opacity: 0; }
    23% { opacity: 1; }
    100% { opacity: 0; }
  }

  @keyframes meter-wait {
    from { opacity: 0.55; transform: scaleY(calc(var(--bar-scale) * 0.85)); }
    to { opacity: 0.92; transform: scaleY(calc(var(--bar-scale) * 1.25)); }
  }

  @keyframes meter-polish {
    0% { opacity: 0; transform: translateX(-80%); }
    50% { opacity: 0.22; }
    100% { opacity: 0; transform: translateX(80%); }
  }

  @keyframes caption-text-in {
    from { opacity: 0.82; }
    to { opacity: 1; }
  }

  @media (prefers-reduced-motion: reduce) {
    .overlay-caption.visible,
    .overlay-caption.dismissing,
    .overlay-caption.dismissing::after,
    [data-phase="waiting_final_result"] .overlay-voice-meter i,
    [data-phase="post_editing"] .overlay-voice-meter::before,
    .overlay-caption-text span {
      animation: none;
    }

    .overlay-voice-meter,
    .overlay-voice-meter i {
      transition: none;
    }
  }
</style>
