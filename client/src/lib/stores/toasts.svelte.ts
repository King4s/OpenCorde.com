/**
 * @file toasts.svelte.ts — Global toast notification store
 * @purpose Manage ephemeral toast messages with auto-dismiss
 * @used-by ToastContainer.svelte, api/client.ts permission-denied handling
 */

export type ToastType = "info" | "error" | "warning" | "success";

export interface Toast {
  id: string;
  type: ToastType;
  title?: string;
  message: string;
  durationMs: number;
}

let toasts = $state<Toast[]>([]);

let nextId = 0;
function makeId(): string {
  return `toast_${++nextId}_${Date.now()}`;
}

/** Show a new toast. Returns the toast id so it can be dismissed early. */
export function addToast(
  message: string,
  opts?: { type?: ToastType; title?: string; durationMs?: number },
): string {
  const id = makeId();
  const toast: Toast = {
    id,
    type: opts?.type ?? "info",
    title: opts?.title,
    message,
    durationMs: opts?.durationMs ?? 5000,
  };
  toasts.push(toast);
  // Auto-remove after duration
  setTimeout(() => {
    removeToast(id);
  }, toast.durationMs);
  return id;
}

export function removeToast(id: string) {
  toasts = toasts.filter((t) => t.id !== id);
}

/** Reactive accessor for the current toast list. */
export function getToasts(): Toast[] {
  return toasts;
}

/** Convenience helpers */
export function toastError(message: string, title?: string) {
  return addToast(message, { type: "error", title, durationMs: 6000 });
}
export function toastWarning(message: string, title?: string) {
  return addToast(message, { type: "warning", title, durationMs: 5000 });
}
export function toastSuccess(message: string, title?: string) {
  return addToast(message, { type: "success", title, durationMs: 3000 });
}
export function toastInfo(message: string, title?: string) {
  return addToast(message, { type: "info", title, durationMs: 4000 });
}
