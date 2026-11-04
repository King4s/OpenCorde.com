<script lang="ts">
  /**
   * @file StatusPicker — full presence status picker with custom status and activity
   * @purpose Dropdown/modal for setting online status, custom status (text+emoji+expiry), and activity
   * @depends $lib/stores/presence
   */
  import { currentUser } from "$lib/stores/auth";
  import {
    updateMyPresence,
    clearCustomStatus,
    STATUS_LABELS,
    STATUS_COLORS,
    ACTIVITY_LABELS,
    numericToStatus,
    type PresenceStatus,
    type ActivityType,
    type UpdatePresencePayload,
  } from "$lib/stores/presence";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  // ── Local state ─────────────────────────────────────────────────

  let currentStatus = $derived(numericToStatus($currentUser?.status ?? 0));
  let currentCustomText = $derived($currentUser?.custom_status_text ?? "");
  let currentActivityType = $derived(($currentUser?.activity_type ?? "") as ActivityType | "");
  let currentActivityName = $derived($currentUser?.activity_name ?? "");

  let customText = $state($currentUser?.custom_status_text ?? "");
  let customEmoji = $state("");
  let customExpiry = $state("never"); // "30m" | "1h" | "4h" | "today" | "never"
  let activityType = $state<ActivityType | "">(($currentUser?.activity_type ?? "") as ActivityType | "");
  let activityName = $state($currentUser?.activity_name ?? "");

  let saved = $state(false);
  let saving = $state(false);
  let error = $state("");

  // ── Expiry options ──────────────────────────────────────────────

  const expiryOptions = [
    { value: "30m", label: "30 minutes" },
    { value: "1h", label: "1 hour" },
    { value: "4h", label: "4 hours" },
    { value: "today", label: "Today" },
    { value: "never", label: "Don't clear" },
  ];

  function computeExpiresAt(): string | undefined {
    if (customExpiry === "never" || !customText.trim()) return undefined;
    const now = new Date();
    switch (customExpiry) {
      case "30m": now.setMinutes(now.getMinutes() + 30); break;
      case "1h": now.setHours(now.getHours() + 1); break;
      case "4h": now.setHours(now.getHours() + 4); break;
      case "today":
        now.setHours(23, 59, 59, 999);
        break;
    }
    return now.toISOString();
  }

  // ── Status selection ────────────────────────────────────────────

  const statuses: PresenceStatus[] = ["online", "idle", "dnd", "invisible"];

  async function selectStatus(status: PresenceStatus) {
    saving = true;
    error = "";
    try {
      const statusNum = status === "online" ? 0 : status === "idle" ? 1 : status === "dnd" ? 2 : 3;
      const payload: UpdatePresencePayload = { status: statusNum };

      // Preserve current custom status / activity when only changing status
      if (customText.trim()) {
        const emojiPrefix = customEmoji ? `${customEmoji} ` : "";
        payload.custom_status_text = `${emojiPrefix}${customText.trim()}`;
        payload.custom_status_expires_at = computeExpiresAt();
      }
      if (activityType && activityName.trim()) {
        payload.activity_type = activityType;
        payload.activity_name = activityName.trim();
      }

      await updateMyPresence(payload);
      saved = true;
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to update status";
    } finally {
      saving = false;
    }
  }

  // ── Save custom status ──────────────────────────────────────────

  async function saveCustomStatus() {
    if (!customText.trim()) return;
    saving = true;
    error = "";
    try {
      const emojiPrefix = customEmoji ? `${customEmoji} ` : "";
      const payload: UpdatePresencePayload = {
        custom_status_text: `${emojiPrefix}${customText.trim()}`,
        custom_status_expires_at: computeExpiresAt(),
      };
      await updateMyPresence(payload);
      saved = true;
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to save custom status";
    } finally {
      saving = false;
    }
  }

  async function handleClearCustom() {
    saving = true;
    error = "";
    try {
      await clearCustomStatus();
      customText = "";
      customEmoji = "";
      saved = true;
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to clear custom status";
    } finally {
      saving = false;
    }
  }

  // ── Save activity ───────────────────────────────────────────────

  async function saveActivity() {
    saving = true;
    error = "";
    try {
      const payload: UpdatePresencePayload = {};
      if (activityType && activityName.trim()) {
        payload.activity_type = activityType;
        payload.activity_name = activityName.trim();
      } else {
        // Clear activity by sending empty strings
        payload.activity_type = "";
        payload.activity_name = "";
      }
      await updateMyPresence(payload);
      saved = true;
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to save activity";
    } finally {
      saving = false;
    }
  }

  async function handleClearActivity() {
    activityType = "";
    activityName = "";
    saving = true;
    error = "";
    try {
      await updateMyPresence({ activity_type: "", activity_name: "" });
      saved = true;
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to clear activity";
    } finally {
      saving = false;
    }
  }

  // ── Escape key ──────────────────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Overlay -->
<div role="presentation" class="popover-overlay" onclick={onClose}></div>

<div class="status-picker" role="dialog" aria-label="Set Status">
  <!-- Header -->
  <div class="picker-header">
    <h3 class="picker-title">Set Status</h3>
    <button class="close-btn" onclick={onClose} aria-label="Close">×</button>
  </div>

  {#if error}
    <div class="error-msg">{error}</div>
  {/if}
  {#if saved}
    <div class="success-msg">Status updated!</div>
  {/if}

  <div class="picker-body">
    <!-- ── Status Options ── -->
    <div class="section">
      <div class="section-label">STATUS</div>
      {#each statuses as status}
        <button
          class="status-option"
          class:selected={currentStatus === status && !saving}
          onclick={() => selectStatus(status)}
          disabled={saving}
        >
          <span
            class="status-dot"
            style="background-color: {STATUS_COLORS[status]}"
          ></span>
          <div class="status-info">
            <div class="status-name">{STATUS_LABELS[status]}</div>
            {#if status === "invisible"}
              <div class="status-desc">You won't appear online, but will still have full access.</div>
            {/if}
          </div>
          {#if currentStatus === status}
            <span class="check">✓</span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- ── Custom Status ── -->
    <div class="section">
      <div class="section-label">CUSTOM STATUS</div>
      <div class="custom-row">
        <input
          type="text"
          class="emoji-input"
          placeholder="😊"
          maxlength="8"
          bind:value={customEmoji}
          disabled={saving}
        />
        <input
          type="text"
          class="text-input"
          placeholder="What's on your mind?"
          maxlength="128"
          bind:value={customText}
          disabled={saving}
        />
      </div>
      {#if customText.trim()}
        <div class="expiry-row">
          <label class="expiry-label">
            Clear after:
            <select class="expiry-select" bind:value={customExpiry} disabled={saving}>
            {#each expiryOptions as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
          </label>
        </div>
        <div class="action-row">
          <button class="btn btn-primary" onclick={saveCustomStatus} disabled={saving}>
            {saving ? "Saving..." : "Save"}
          </button>
          {#if currentCustomText}
            <button class="btn btn-danger" onclick={handleClearCustom} disabled={saving}>
              Clear
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <!-- ── Activity ── -->
    <div class="section">
      <div class="section-label">ACTIVITY</div>
      <div class="activity-row">
        <select class="type-select" bind:value={activityType} disabled={saving}>
          <option value="">None</option>
          <option value="playing">Playing</option>
          <option value="listening">Listening to</option>
          <option value="streaming">Streaming</option>
          <option value="custom">Custom</option>
        </select>
        {#if activityType}
          <input
            type="text"
            class="text-input"
            placeholder={activityType === "playing"
              ? "Game name"
              : activityType === "listening"
                ? "Song name"
                : activityType === "streaming"
                  ? "Stream title"
                  : "Activity name"}
            maxlength="128"
            bind:value={activityName}
            disabled={saving}
          />
        {/if}
      </div>
      {#if activityType}
        <div class="action-row">
          <button
            class="btn btn-primary"
            onclick={saveActivity}
            disabled={saving || !activityName.trim()}
          >
            {saving ? "Saving..." : "Save Activity"}
          </button>
          {#if currentActivityType}
            <button class="btn btn-danger" onclick={handleClearActivity} disabled={saving}>
              Clear
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .popover-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    z-index: 40;
  }

  .status-picker {
    position: absolute;
    bottom: 60px;
    left: 8px;
    right: 8px;
    background: #111214;
    border-radius: 8px;
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.3);
    border: 1px solid #1f2023;
    z-index: 50;
    max-height: 80vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 8px;
  }

  .picker-title {
    font-size: 14px;
    font-weight: 600;
    color: #f2f3f5;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    color: #949ba4;
    font-size: 20px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }
  .close-btn:hover { color: #f2f3f5; }

  .error-msg {
    padding: 6px 16px;
    font-size: 12px;
    color: #f23f43;
  }

  .success-msg {
    padding: 6px 16px;
    font-size: 12px;
    color: #23a55a;
  }

  .picker-body {
    padding: 0 8px 12px;
  }

  .section {
    margin-top: 8px;
  }

  .section-label {
    font-size: 11px;
    font-weight: 700;
    color: #949ba4;
    text-transform: uppercase;
    padding: 4px 8px;
    margin-bottom: 2px;
  }

  .status-option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 4px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;
  }
  .status-option:hover { background: #35373c; }
  .status-option.selected { background: #2b2d31; }
  .status-option:disabled { opacity: 0.6; cursor: default; }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .status-name { font-size: 13px; font-weight: 500; color: #f2f3f5; }
  .status-desc { font-size: 10px; color: #949ba4; white-space: normal; line-height: 1.2; margin-top: 2px; }

  .check {
    color: #23a55a;
    font-size: 14px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .custom-row {
    display: flex;
    gap: 6px;
    padding: 0 8px;
  }

  .emoji-input {
    width: 44px;
    padding: 6px;
    font-size: 18px;
    text-align: center;
    background: #1e1f22;
    border: 1px solid #2b2d31;
    border-radius: 4px;
    color: #f2f3f5;
  }
  .emoji-input:focus {
    outline: none;
    border-color: #5865f2;
  }

  .text-input {
    flex: 1;
    padding: 6px 8px;
    font-size: 13px;
    background: #1e1f22;
    border: 1px solid #2b2d31;
    border-radius: 4px;
    color: #f2f3f5;
  }
  .text-input:focus {
    outline: none;
    border-color: #5865f2;
  }

  .expiry-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 0;
  }

  .expiry-label {
    font-size: 11px;
    color: #949ba4;
    white-space: nowrap;
  }

  .expiry-select {
    flex: 1;
    padding: 4px 6px;
    font-size: 12px;
    background: #1e1f22;
    border: 1px solid #2b2d31;
    border-radius: 4px;
    color: #f2f3f5;
  }

  .type-select {
    padding: 6px 8px;
    font-size: 13px;
    background: #1e1f22;
    border: 1px solid #2b2d31;
    border-radius: 4px;
    color: #f2f3f5;
  }

  .activity-row {
    display: flex;
    gap: 6px;
    padding: 0 8px;
  }

  .action-row {
    display: flex;
    gap: 6px;
    padding: 6px 8px 0;
  }

  .btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .btn-primary {
    background: #5865f2;
    color: white;
  }
  .btn-primary:hover:not(:disabled) { background: #4752c4; }
  .btn-danger {
    background: #35373c;
    color: #f23f43;
  }
  .btn-danger:hover:not(:disabled) { background: #4a1b1d; }
</style>
