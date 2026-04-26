<script lang="ts">
  import { AlertCircle, CheckCircle2, ChevronRight, LoaderCircle, RefreshCw } from "lucide-svelte";

  export type SetupStatusItem = {
    label: string;
    value: string;
    ok: boolean;
    checking?: boolean;
    action: string;
  };

  export type SetupWarningLevel = "blocking" | "warning" | "info";

  export type SetupStatusWarning = {
    code: string;
    level?: SetupWarningLevel;
    title: string;
    message: string;
    action: string;
  };

  type Texts = {
    title: string;
    pendingTitle: string;
    pendingDescription: string;
    checkingTitle: string;
    checkingDescription: string;
    readyTitle: string;
    readyDescription: string;
    refresh: string;
    warningSummary: (count: number) => string;
    actionText: (action: string) => string;
  };

  type Props = {
    ready: boolean;
    checking: boolean;
    items: SetupStatusItem[];
    warnings: SetupStatusWarning[];
    texts: Texts;
    onAction: (action: string) => void;
    onRefresh: () => void;
  };

  let { ready, checking, items, warnings, texts, onAction, onRefresh }: Props = $props();

  function warningLevel(warning: SetupStatusWarning): SetupWarningLevel {
    return warning.level ?? "blocking";
  }

  let blockingWarnings = $derived(warnings.filter((warning) => warningLevel(warning) === "blocking"));
  let softWarnings = $derived(warnings.filter((warning) => warningLevel(warning) === "warning"));
</script>

<section class:ready={ready && !checking} class:checking class="setup-status-card">
  <div class="setup-status-head">
    <div>
      <span>{texts.title}</span>
      <strong>{checking ? texts.checkingTitle : ready ? texts.readyTitle : texts.pendingTitle}</strong>
      <p>{checking ? texts.checkingDescription : ready ? texts.readyDescription : texts.pendingDescription}</p>
    </div>
    <button type="button" onclick={onRefresh}>
      <RefreshCw size={15} />
      {texts.refresh}
    </button>
  </div>

  <div class="setup-check-grid">
    {#each items as item}
      <button
        type="button"
        class:ok={item.ok}
        class:checking={item.checking}
        class="setup-check-item"
        onclick={() => onAction(item.action)}
        disabled={item.checking}
      >
        <span class="setup-check-icon">
          {#if item.checking}
            <LoaderCircle size={17} />
          {:else if item.ok}
            <CheckCircle2 size={17} />
          {:else}
            <AlertCircle size={17} />
          {/if}
        </span>
        <span>
          <strong>{item.label}</strong>
          <small>{item.value}</small>
        </span>
        <ChevronRight size={15} />
      </button>
    {/each}
  </div>

  {#if blockingWarnings.length > 0}
    <div class="setup-warning-list blocking">
      {#each blockingWarnings as warning}
        <article>
          <div>
            <strong>{warning.title}</strong>
            <p>{warning.message}</p>
          </div>
          <button type="button" onclick={() => onAction(warning.action)}>
            {texts.actionText(warning.action)}
          </button>
        </article>
      {/each}
    </div>
  {/if}

  {#if softWarnings.length > 0}
    <details class="setup-warning-details">
      <summary>
        <AlertCircle size={15} />
        <span>{texts.warningSummary(softWarnings.length)}</span>
      </summary>
      <div class="setup-warning-list soft">
        {#each softWarnings as warning}
          <article>
            <div>
              <strong>{warning.title}</strong>
              <p>{warning.message}</p>
            </div>
            <button type="button" onclick={() => onAction(warning.action)}>
              {texts.actionText(warning.action)}
            </button>
          </article>
        {/each}
      </div>
    </details>
  {/if}

</section>

<style>
  .setup-status-card {
    display: grid;
    gap: 12px;
    padding: 14px;
    border: 1px solid #d9e7f7;
    border-radius: 8px;
    background: linear-gradient(180deg, #ffffff 0%, #f8fbff 100%);
    box-shadow: 0 14px 34px rgba(28, 56, 96, 0.08);
  }

  .setup-status-card.ready {
    border-color: rgba(25, 135, 84, 0.28);
    background: linear-gradient(180deg, #ffffff 0%, #f5fff9 100%);
  }

  .setup-status-card.checking {
    border-color: rgba(100, 116, 139, 0.22);
    background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
  }

  .setup-status-head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .setup-status-head > div {
    flex: 1 1 320px;
    min-width: 0;
  }

  .setup-status-head span {
    display: block;
    color: #2f80ed;
    font-size: 12px;
    font-weight: 800;
  }

  .setup-status-head strong {
    display: block;
    margin-top: 3px;
    color: #132033;
    font-size: 18px;
    line-height: 1.2;
  }

  .setup-status-head p {
    margin: 5px 0 0;
    color: #66758a;
    font-size: 13px;
    line-height: 1.45;
  }

  .setup-status-head button,
  .setup-warning-list button {
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 32px;
    padding: 0 11px;
    border: 1px solid #d7e3f2;
    border-radius: 8px;
    background: #ffffff;
    color: #2b5d9b;
    font-size: 12px;
    font-weight: 800;
    cursor: pointer;
  }

  .setup-status-head button:focus-visible,
  .setup-warning-list button:focus-visible,
  .setup-warning-details summary:focus-visible,
  .setup-check-item:focus-visible {
    outline: 2px solid rgba(47, 128, 237, 0.32);
    outline-offset: 2px;
  }

  .setup-check-grid {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 8px;
  }

  .setup-check-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-height: 56px;
    padding: 9px;
    border: 1px solid #e0e9f5;
    border-radius: 8px;
    background: #ffffff;
    text-align: left;
    cursor: pointer;
  }

  .setup-check-item > span:nth-child(2) {
    min-width: 0;
  }

  .setup-check-item.ok .setup-check-icon {
    color: #198754;
    background: #eaf8ef;
  }

  .setup-check-item.checking .setup-check-icon {
    color: #64748b;
    background: #f1f5f9;
  }

  .setup-check-item.checking .setup-check-icon :global(svg) {
    animation: setup-spin 900ms linear infinite;
  }

  .setup-check-item:disabled {
    cursor: wait;
  }

  .setup-check-icon {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border-radius: 8px;
    color: #d97706;
    background: #fff5e6;
  }

  .setup-check-item strong,
  .setup-check-item small {
    display: block;
    min-width: 0;
    line-height: 1.28;
    overflow-wrap: anywhere;
  }

  .setup-check-item strong {
    color: #1b2533;
    font-size: 13px;
  }

  .setup-check-item small {
    margin-top: 3px;
    color: #65758b;
    font-size: 12px;
  }

  .setup-warning-list {
    display: grid;
    gap: 8px;
  }

  .setup-warning-details {
    overflow: hidden;
    border: 1px solid #e1eaf6;
    border-radius: 8px;
    background: #ffffff;
  }

  .setup-warning-details summary {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    padding: 0 12px;
    color: #5d6f87;
    font-size: 12px;
    font-weight: 800;
    cursor: pointer;
    list-style: none;
  }

  .setup-warning-details summary::-webkit-details-marker {
    display: none;
  }

  .setup-warning-details[open] summary {
    border-bottom: 1px solid #e7eef8;
  }

  .setup-warning-details .setup-warning-list {
    padding: 8px;
  }

  .setup-warning-list article {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 10px 12px;
    border: 1px solid #f4d7ad;
    border-radius: 8px;
    background: #fffaf3;
  }

  .setup-warning-list.soft article {
    border-color: #e1eaf6;
    background: #fbfdff;
  }

  .setup-warning-list article > div {
    min-width: 0;
  }

  .setup-warning-list strong {
    color: #8a4b00;
    font-size: 13px;
  }

  .setup-warning-list.soft strong {
    color: #3d4c61;
  }

  .setup-warning-list p {
    margin: 3px 0 0;
    color: #715536;
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .setup-warning-list.soft p {
    color: #65758b;
  }

  .setup-warning-list button {
    max-width: 180px;
    white-space: normal;
  }

  @media (max-width: 1180px) {
    .setup-check-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .setup-check-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .setup-warning-list article {
      display: grid;
      align-items: stretch;
    }

    .setup-warning-list button {
      width: 100%;
      max-width: none;
    }
  }

  @media (max-width: 520px) {
    .setup-check-grid {
      grid-template-columns: 1fr;
    }
  }

  @keyframes setup-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
