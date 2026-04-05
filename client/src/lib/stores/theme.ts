/**
 * @file Theme store — manages light/dark theme preference and message style
 * @purpose Provides global theme state, toggle functionality, and message display preferences
 */
import { writable, type Writable } from "svelte/store";

const THEME_KEY = "opencorde_theme";
const MESSAGE_STYLE_KEY = "opencorde_message_style";

type Theme = "dark" | "light";
type MessageStyle = "cozy" | "compact";

function getInitialTheme(): Theme {
  if (typeof localStorage !== "undefined") {
    return (localStorage.getItem(THEME_KEY) as Theme) ?? "dark";
  }
  return "dark";
}

function getInitialMessageStyle(): MessageStyle {
  if (typeof localStorage !== "undefined") {
    return (localStorage.getItem(MESSAGE_STYLE_KEY) as MessageStyle) ?? "cozy";
  }
  return "cozy";
}

// Create internal writable stores
const internalTheme: Writable<Theme> = writable(getInitialTheme());
const internalMessageStyle: Writable<MessageStyle> = writable(
  getInitialMessageStyle(),
);

// Subscribe to persist changes
internalTheme.subscribe((value) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(THEME_KEY, value);
  }
  applyTheme(value);
});

internalMessageStyle.subscribe((value) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(MESSAGE_STYLE_KEY, value);
  }
});

function applyTheme(theme: Theme) {
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("data-theme", theme);
  }
}

// Public API: themeStore is both a store and has helper methods
export const themeStore = {
  // Store interface for subscriptions (for use with $)
  subscribe: internalTheme.subscribe,

  // Derived store for message style
  messageStyle: internalMessageStyle,

  // Method to toggle theme
  toggle() {
    internalTheme.update((t) => (t === "dark" ? "light" : "dark"));
  },

  // Method to toggle message style
  toggleMessageStyle() {
    internalMessageStyle.update((m) => (m === "cozy" ? "compact" : "cozy"));
  },

  // Initialize on app load
  init() {
    let currentTheme: Theme = "dark";
    internalTheme.subscribe((t) => {
      currentTheme = t;
    })();
    applyTheme(currentTheme);
  },

  // Getter for isDark (for backward compatibility)
  get isDark() {
    let value = true;
    internalTheme.subscribe((t) => {
      value = t === "dark";
    })();
    return value;
  },
};
