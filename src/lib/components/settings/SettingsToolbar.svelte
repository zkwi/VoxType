<script lang="ts">
  import { Save, ShieldCheck } from "lucide-svelte";

  type Props = {
    title: string;
    hint: string;
    statusMessage: string;
    saveLabel: string;
    savingLabel: string;
    reloadLabel: string;
    saving: boolean;
    dirty?: boolean;
    onSave: () => void;
    onReload: () => void;
  };

  let {
    title,
    hint,
    statusMessage,
    saveLabel,
    savingLabel,
    reloadLabel,
    saving,
    dirty = false,
    onSave,
    onReload,
  }: Props = $props();
</script>

<section class:dirty class="settings-toolbar" aria-label={title}>
  <div>
    <strong>{title}</strong>
    <span>{statusMessage || hint}</span>
  </div>
  <div class="toolbar-actions">
    <button type="button" class="primary" onclick={onSave} disabled={saving}>
      <Save size={16} />
      {saving ? savingLabel : saveLabel}
    </button>
    <button type="button" onclick={onReload}>
      <ShieldCheck size={16} />
      {reloadLabel}
    </button>
  </div>
</section>

<style>
  .settings-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    background: rgba(255, 255, 255, 0.95);
    border: 1px solid var(--border, #dde6f3);
    border-radius: 16px;
    box-shadow: 0 12px 34px rgba(15, 23, 42, 0.1);
    backdrop-filter: blur(14px);
  }

  .settings-toolbar.dirty {
    border-color: rgba(47, 128, 237, 0.38);
    box-shadow: 0 14px 36px rgba(47, 128, 237, 0.14);
  }

  .settings-toolbar strong {
    display: block;
    color: var(--text-main, #111827);
    font-size: 14px;
    font-weight: 800;
  }

  .settings-toolbar span {
    display: block;
    margin-top: 3px;
    color: var(--text-secondary, #64748b);
    font-size: 12px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .toolbar-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    justify-content: flex-end;
  }

  .toolbar-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: 108px;
    max-width: 100%;
    min-height: 36px;
    padding: 0 16px;
    color: var(--text-main, #111827);
    background: #ffffff;
    border: 1px solid var(--border, #dde6f3);
    border-radius: 12px;
    font-weight: 800;
    cursor: pointer;
    white-space: nowrap;
  }

  .toolbar-actions .primary {
    color: #ffffff;
    background: var(--primary, #2f80ed);
    border-color: var(--primary, #2f80ed);
  }

  .settings-toolbar.dirty .toolbar-actions .primary {
    background: #1d4ed8;
    border-color: #1d4ed8;
  }

  .toolbar-actions button:disabled {
    cursor: wait;
    opacity: 0.66;
  }

  .toolbar-actions button:focus-visible {
    outline: 2px solid rgba(47, 128, 237, 0.32);
    outline-offset: 2px;
  }

  @media (max-width: 920px) {
    .settings-toolbar {
      grid-template-columns: 1fr;
      align-items: stretch;
    }

    .toolbar-actions {
      justify-content: stretch;
    }

    .toolbar-actions button {
      flex: 1 1 150px;
    }
  }
</style>
