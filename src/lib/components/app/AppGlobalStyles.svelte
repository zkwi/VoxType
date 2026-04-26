<style>
  :global(:root) {
    --blue-50: #eef8ff;
    --blue-100: #d9efff;
    --blue-200: #c7e6fb;
    --blue-500: #2f8cff;
    --blue-600: #176ee6;
    --blue-700: #1158bb;
    --ink: #202933;
    --muted: #7a8794;
    --line: #e7edf3;
    --panel: #ffffff;
    --canvas: #f6f8fb;
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    margin: 0;
    min-width: 320px;
    width: 100%;
    height: 100%;
    overflow: hidden;
    color: var(--ink);
    background: #e9eef5;
    font-family: "Segoe UI", "Microsoft YaHei", sans-serif;
    font-size: 16px;
    letter-spacing: 0;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
  }

  :global(html:has(.overlay-root)),
  :global(body:has(.overlay-root)) {
    min-width: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden !important;
    background: transparent;
  }

  :global(html:has(.toast-root)),
  :global(body:has(.toast-root)) {
    min-width: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden !important;
    background: transparent;
    font-size: 14px;
  }

  :global(body:has(.overlay-root)::-webkit-scrollbar),
  :global(body:has(.overlay-root) *::-webkit-scrollbar),
  :global(body:has(.toast-root)::-webkit-scrollbar),
  :global(body:has(.toast-root) *::-webkit-scrollbar) {
    width: 0;
    height: 0;
    display: none;
  }

  :global(button),
  :global(input),
  :global(textarea),
  :global(select) {
    font: inherit;
    transition: all 160ms ease;
  }

  :global(button) {
    cursor: pointer;
    border: 0;
    background: transparent;
  }

  :global(:root) {
    --primary: #2f80ed;
    --primary-hover: #256fe0;
    --primary-light: #eaf3ff;
    --gradient-start: #2f80ed;
    --gradient-end: #7c3aed;
    --text-main: #111827;
    --text-secondary: #64748b;
    --text-muted: #94a3b8;
    --bg-page: #f8fafc;
    --bg-sidebar: #f3f8ff;
    --bg-card: #ffffff;
    --border: #dde6f3;
    --border-strong: #cbd5e1;
    --success: #10b981;
    --warning: #f59e0b;
    --danger: #ef4444;
    --radius-sm: 8px;
    --radius-md: 12px;
    --radius-lg: 16px;
    --radius-xl: 24px;
    --shadow-card: 0 8px 24px rgba(15, 23, 42, 0.06);
    --shadow-soft: 0 4px 12px rgba(15, 23, 42, 0.08);
  }

  :global(html:not(:has(.overlay-root)):not(:has(.toast-root))),
  :global(body:not(:has(.overlay-root)):not(:has(.toast-root))) {
    background: var(--bg-page);
    font-family: "Microsoft YaHei", "Segoe UI", "PingFang SC", "SF Pro Display", "Noto Sans CJK SC", sans-serif;
  }
</style>
