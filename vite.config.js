import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST || "127.0.0.1";

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit({
    preprocess: vitePreprocess(),
    // Tauri bundles a static SPA without a Node.js server.
    adapter: adapter({ fallback: "index.html" }),
    version: { pollInterval: 0 },
  })],
  resolve: { alias: { $lib: fileURLToPath(new URL("./src/lib", import.meta.url)) } },
  test: {
    include: ["src/lib/utils/**/*.test.ts"],
    environment: "node",
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  build: {
    target: "chrome90",
  },
  server: {
    port: 18080,
    strictPort: true,
    host,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 18080,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
