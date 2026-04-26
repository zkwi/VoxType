<script lang="ts">
  import { AlertCircle, Check, Info } from "lucide-svelte";

  export type ActionNoticeKind = "success" | "info" | "warning" | "error";

  type Props = {
    message?: string | null;
    kind?: ActionNoticeKind;
  };

  let { message = null, kind = "info" }: Props = $props();
</script>

{#if message}
  <div
    class:error={kind === "error"}
    class:info={kind === "info"}
    class:warning={kind === "warning"}
    class="action-notice"
    role="status"
    aria-live="polite"
  >
    {#if kind === "success"}
      <Check size={16} />
    {:else if kind === "info"}
      <Info size={16} />
    {:else}
      <AlertCircle size={16} />
    {/if}
    <span>{message}</span>
  </div>
{/if}

<style>
  .action-notice {
    position: fixed;
    right: 22px;
    bottom: 20px;
    z-index: 20;
    display: inline-flex;
    align-items: flex-start;
    max-width: min(460px, calc(100vw - 44px));
    min-height: 40px;
    gap: 8px;
    padding: 10px 14px;
    color: #0f5132;
    background: rgba(240, 253, 244, 0.98);
    border: 1px solid rgba(34, 197, 94, 0.26);
    border-radius: 12px;
    box-shadow: 0 14px 34px rgba(15, 23, 42, 0.12);
    font-size: 14px;
    font-weight: 700;
  }

  .action-notice span {
    min-width: 0;
    line-height: 1.4;
    overflow-wrap: anywhere;
    white-space: normal;
  }

  .action-notice.info {
    color: #245b93;
    background: rgba(240, 247, 255, 0.98);
    border-color: rgba(47, 128, 237, 0.24);
  }

  .action-notice.warning {
    color: #854d0e;
    background: rgba(255, 251, 235, 0.98);
    border-color: rgba(245, 158, 11, 0.32);
  }

  .action-notice.error {
    color: #991b1b;
    background: rgba(254, 242, 242, 0.98);
    border-color: rgba(239, 68, 68, 0.3);
  }

  @media (prefers-reduced-motion: no-preference) {
    .action-notice {
      animation: action-notice-enter 180ms ease-out;
    }

    @keyframes action-notice-enter {
      from {
        opacity: 0;
        transform: translateY(8px);
      }

      to {
        opacity: 1;
        transform: translateY(0);
      }
    }
  }
</style>
