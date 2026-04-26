<script lang="ts">
  import type { OverlayMode } from "$lib/types/app";

  type Props = {
    meterBars: number[];
    displayLines: string[];
    recording: boolean;
    mode: OverlayMode;
    fontSize: number;
    rootStyle: string;
    meterBarHeight: (bar: number) => string;
    meterBarOpacity: (bar: number) => string;
    textElement?: HTMLDivElement | null;
  };

  let {
    meterBars,
    displayLines,
    recording,
    mode,
    fontSize,
    rootStyle,
    meterBarHeight,
    meterBarOpacity,
    textElement = $bindable<HTMLDivElement | null>(null),
  }: Props = $props();
</script>

<main class="overlay-root" style={rootStyle}>
  <div class="overlay-caption">
    <div class:active={recording} class="overlay-voice-meter" aria-hidden="true">
      {#each meterBars as bar}
        <i style:height={meterBarHeight(bar)} style:opacity={meterBarOpacity(bar)}></i>
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
    padding: 0;
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
    padding: 8px 18px;
    overflow: hidden;
    color: var(--overlay-text, #ffffff);
    background: rgba(var(--overlay-bg-rgb, 23, 110, 230), var(--overlay-opacity, 0.9));
    border: 0;
    border-radius: 0;
    box-shadow: none;
    font-family: "Microsoft YaHei", "Segoe UI", "PingFang SC", "SF Pro Display", "Noto Sans CJK SC", sans-serif;
    text-align: left;
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
    border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    border-radius: 999px;
    opacity: 0.72;
    pointer-events: none;
    transition: opacity 140ms ease, background 140ms ease, border-color 140ms ease;
  }

  .overlay-voice-meter.active {
    opacity: 0.88;
  }

  .overlay-voice-meter i {
    width: 2px;
    min-height: 4px;
    background: currentColor;
    border-radius: 999px;
    transition: height 120ms ease, opacity 120ms ease;
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
</style>
