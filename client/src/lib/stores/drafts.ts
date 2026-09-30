/**
 * @file Draft store — persists unsent message text per channel, thread, and DM
 * @purpose localStorage-backed draft persistence with debounced writes and TTL cleanup
 * @depends svelte environment (browser check)
 * @version 1.0.0
 */
import { browser } from "$app/environment";

const DRAFTS_KEY = "opencorde_drafts";
const DEBOUNCE_MS = 300;
const MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000; // 30 days

interface DraftEntry {
  content: string;
  updatedAt: number;
}

let cache: Record<string, DraftEntry> | null = null;
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function load(): Record<string, DraftEntry> {
  if (!browser) return {};
  if (cache) return cache;
  try {
    const raw = localStorage.getItem(DRAFTS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, DraftEntry>;
    const cutoff = Date.now() - MAX_AGE_MS;
    const cleaned: Record<string, DraftEntry> = {};
    for (const [k, v] of Object.entries(parsed)) {
      if (v.updatedAt > cutoff) cleaned[k] = v;
    }
    cache = cleaned;
    return cleaned;
  } catch {
    return {};
  }
}

function persist() {
  if (!browser || !cache) return;
  localStorage.setItem(DRAFTS_KEY, JSON.stringify(cache));
}

/** Get the persisted draft content for a key, or empty string. */
export function getDraft(key: string): string {
  return load()[key]?.content ?? "";
}

/** Persist draft content for a key (debounced). */
export function setDraft(key: string, content: string) {
  if (!browser || !key) return;
  const drafts = load();
  drafts[key] = { content, updatedAt: Date.now() };
  cache = drafts;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(persist, DEBOUNCE_MS);
}

/** Remove a draft key from storage (debounced). */
export function clearDraft(key: string) {
  if (!browser || !key) return;
  const drafts = load();
  delete drafts[key];
  cache = drafts;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(persist, DEBOUNCE_MS);
}
