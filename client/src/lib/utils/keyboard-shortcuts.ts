/**
 * @file keyboard-shortcuts — Global keyboard shortcut handler
 * @purpose Registers app-wide keyboard shortcuts for navigation and common actions
 */

import { goto } from "$app/navigation";

export interface Shortcut {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  description: string;
  action: () => void;
}

let shortcuts: Shortcut[] = [];
let enabled = true;

export function registerShortcuts(newShortcuts: Shortcut[]) {
  shortcuts = newShortcuts;
}

export function enableShortcuts() {
  enabled = true;
}

export function disableShortcuts() {
  enabled = false;
}

export function handleKeydown(e: KeyboardEvent, spaceId?: string) {
  if (!enabled) return;

  // Don't fire when typing in inputs/textareas
  const target = e.target as HTMLElement;
  if (
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.isContentEditable
  ) {
    // Only allow Escape from inputs
    if (e.key !== "Escape") return;
  }

  // App-level shortcuts
  if (e.key === "Escape") {
    // Close any open modals — dispatch custom event
    document.dispatchEvent(new CustomEvent("opencorde:close-modal"));
    return;
  }

  // Ctrl+K — Quick switcher / search
  if (e.ctrlKey && e.key === "k") {
    e.preventDefault();
    document.dispatchEvent(new CustomEvent("opencorde:open-search"));
    return;
  }

  // Ctrl+, — Settings
  if (e.ctrlKey && e.key === ",") {
    e.preventDefault();
    goto("/settings");
    return;
  }

  // Alt+Home — DMs
  if (e.altKey && e.key === "Home") {
    e.preventDefault();
    goto("/@me/dms");
    return;
  }

  // Check registered shortcuts
  for (const shortcut of shortcuts) {
    const ctrlMatch = !!shortcut.ctrl === e.ctrlKey;
    const altMatch = !!shortcut.alt === e.altKey;
    const shiftMatch = !!shortcut.shift === e.shiftKey;
    const keyMatch = e.key.toLowerCase() === shortcut.key.toLowerCase();
    if (ctrlMatch && altMatch && shiftMatch && keyMatch) {
      e.preventDefault();
      shortcut.action();
      return;
    }
  }
}

export const DEFAULT_SHORTCUTS: Shortcut[] = [
  {
    key: "k",
    ctrl: true,
    description: "Quick search",
    action: () =>
      document.dispatchEvent(new CustomEvent("opencorde:open-search")),
  },
  {
    key: ",",
    ctrl: true,
    description: "Open settings",
    action: () => goto("/settings"),
  },
  {
    key: "Escape",
    description: "Close modal / cancel",
    action: () =>
      document.dispatchEvent(new CustomEvent("opencorde:close-modal")),
  },
  {
    key: "Home",
    alt: true,
    description: "Go to DMs",
    action: () => goto("/@me/dms"),
  },
];
