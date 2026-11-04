import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defaultClientConditions, defineConfig } from "vite";

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  // Vite 6 no longer adds browser conditions to vite-plugin-svelte 4's override.
  // Keep lifecycle imports on Svelte's browser runtime in the client build.
  resolve: {
    conditions: [...defaultClientConditions],
  },
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
});
