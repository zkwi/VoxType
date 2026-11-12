<script lang="ts">
  import type { CopyKey } from "$lib/i18n";

  type Props = {
    targets: readonly { id: string; label: CopyKey }[];
    t: (key: CopyKey) => string;
    onJump: (id: string) => void;
  };

  let { targets, t, onJump }: Props = $props();
</script>

<nav class="settings-jump-nav" aria-label={t("settingsQuickNav")}>
  {#each targets as target (target.id)}
    <button type="button" onclick={() => onJump(target.id)}>{t(target.label)}</button>
  {/each}
</nav>

<style>
  .settings-jump-nav {
    position: sticky;
    top: -1px;
    z-index: 4;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(128px, 100%), 1fr));
    gap: 6px;
    padding: 8px;
    background: rgba(247, 250, 254, 0.96);
    border: 1px solid var(--border);
    border-radius: 12px;
    backdrop-filter: blur(12px);
  }

  button {
    min-width: 0;
    min-height: 34px;
    padding: 4px 8px;
    color: var(--text-secondary);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 12px;
    font-weight: 800;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  button:hover,
  button:focus-visible {
    color: var(--primary);
    border-color: var(--primary);
  }
</style>
