/**
 * Vite plugin: injects __BUILD_TIMESTAMP__ into sw.js during build.
 * Each build gets a unique cache name so the old SW cache is purged on activate.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { Plugin } from "vite";

const SW_PATH = resolve(__dirname, "static/sw.js");

export function pwaVersion(): Plugin {
  let outDir = "";

  return {
    name: "pwa-version",
    enforce: "post",

    configResolved(config) {
      outDir = config.build.outDir;
    },

    writeBundle() {
      const ts = Date.now().toString();
      const dest = resolve(outDir, "sw.js");
      try {
        let src = readFileSync(SW_PATH, "utf-8");
        src = src.replace(/__BUILD_TIMESTAMP__/g, ts);
        writeFileSync(dest, src, "utf-8");
        console.log(`[pwa-version] Injected build timestamp ${ts} into sw.js`);
      } catch (err) {
        console.warn("[pwa-version] Failed to process sw.js:", err);
      }
    },
  };
}
