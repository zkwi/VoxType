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
      {#each meterBars as bar, index}
        <i style:--bar-scale={meterBarScale(bar)} style:--bar-delay={`${index * 110}ms`}></i>
      {/each}
    </div>
    <div
      class:single={mode === "single"}
      class:double={mode === "double"}
      class="overlay-caption-text"
      style={`font-size: ${fontSize}px`}
      bind:this={textElement}
    >
      {#each displayLines as line}
        <span>{line || "\u00a0"}</span>
      {/each}
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
    opacity: 0;
    font-family: "Microsoft YaHei", "Segoe UI", "PingFang SC", "SF Pro Display", "Noto Sans CJK SC", sans-serif;
    text-align: left;
  }

  .overlay-caption.visible {
    opacity: 1;
  }

  .overlay-caption.dismissing {
    animation: caption-out 140ms ease-in both;
  }

  .overlay-voice-meter {
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
    border-radius: 999px;
    opacity: 0.72;
    pointer-events: none;
    overflow: hidden;
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
    transform: scaleY(0.28);
  }

  .overlay-caption.visible:not(.dismissing):is([data-phase="waiting_final_result"], [data-phase="post_editing"]) .overlay-voice-meter i {
    animation: meter-processing 950ms ease-in-out var(--bar-delay) infinite alternate;
  }

  [data-phase="post_editing"] .overlay-voice-meter {
    background: color-mix(in srgb, currentColor 22%, transparent);
    opacity: 0.92;
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
  }

  @keyframes caption-out {
    from { opacity: 1; }
    to { opacity: 0; }
  }

  @keyframes meter-processing {
    from { transform: scaleY(0.34); }
    to { transform: scaleY(0.66); }
  }

  @media (prefers-reduced-motion: reduce) {
    .overlay-caption.dismissing,
    .overlay-caption.visible:not(.dismissing):is([data-phase="waiting_final_result"], [data-phase="post_editing"]) .overlay-voice-meter i {
      animation: none;
    }

    .overlay-voice-meter i {
      transition: none;
    }
  }
</style>
