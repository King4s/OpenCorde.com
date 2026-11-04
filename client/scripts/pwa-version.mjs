#!/usr/bin/env node
/**
 * Post-build script: injects __BUILD_TIMESTAMP__ into the built sw.js
 * so each deploy gets a unique cache name → stale caches purged on activate.
 *
 * Run after `vite build` + `adapter-static` (i.e., after `npm run build`).
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const ROOT = resolve(import.meta.dirname, "..");
const SW_DEST = resolve(ROOT, "build/sw.js");
const TS = Date.now().toString();

try {
  let src = readFileSync(SW_DEST, "utf-8");
  if (!src.includes("__BUILD_TIMESTAMP__")) {
    console.log(
      "[pwa-version] No placeholder found — already injected or not a prod build",
    );
    process.exit(0);
  }
  src = src.replace(/__BUILD_TIMESTAMP__/g, TS);
  writeFileSync(SW_DEST, src, "utf-8");
  console.log(`[pwa-version] Injected build timestamp ${TS}`);
} catch (err) {
  console.error("[pwa-version] Failed:", err.message);
  process.exit(1);
}
