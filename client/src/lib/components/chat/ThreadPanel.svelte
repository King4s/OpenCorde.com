<!--
  @component ThreadPanel
  @purpose Slide-in panel for reading and replying to message threads.
  @version 1.0.0
-->
<script lang="ts">
  import { threadStore } from '$lib/stores/threads.svelte';
import { edgeResize } from '$lib/actions/edgeResize';

  let { onClose }: { onClose: () => void } = $props();

  let newMessage = $state('');
  let sending = $state(false);

  async function handleSend() {
    if (!newMessage.trim() || !threadStore.activeThread) return;
    sending = true;
    try {
      await threadStore.sendMessage(threadStore.activeThread.id, newMessage.trim());
      newMessage = '';
    } catch (e: unknown) {
      console.error('Failed to send message', e);
    } finally {
      sending = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function formatTime(ts: string) {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<div use:edgeResize={{ handles: ['left'], minWidth: 260, maxWidth: 520 }} class="thread-panel resizable-thread-panel">
  <div class="thread-header">
    <span class="thread-title">🧵 {threadStore.activeThread?.name ?? 'Thread'}</span>
    <button class="close-btn" onclick={onClose}>✕</button>
  </div>

  <div class="thread-messages">
    {#if threadStore.loading}
      <p class="loading">Loading...</p>
    {:else if threadStore.error}
      <p class="error">{threadStore.error}</p>
    {:else if threadStore.messages.length === 0}
      <p class="empty">No messages yet. Start the thread!</p>
    {:else}
      {#each threadStore.messages as msg (msg.id)}
        <div class="thread-msg">
          <span class="msg-author">{msg.author_username}</span>
          <span class="msg-time">{formatTime(msg.created_at)}</span>
          <p class="msg-content">{msg.content}</p>
        </div>
      {/each}
    {/if}
  </div>

  <div class="thread-input">
    <textarea
      bind:value={newMessage}
      onkeydown={handleKeydown}
      placeholder="Reply in thread..."
      rows="2"
      disabled={sending}
    ></textarea>
    <button
      onclick={handleSend}
      disabled={sending || !newMessage.trim()}
    >
      Send
    </button>
  </div>
</div>

<style>
  .thread-panel {
    width: 320px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #2b2d31;
    border-left: 1px solid #1e1f22;
    flex-shrink: 0;
  }

  .resizable-thread-panel {
    overflow: auto;
    min-width: 260px;
    max-width: 520px;
  }

  .thread-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid #1e1f22;
    font-weight: 600;
    color: #f2f3f5;
  }

  .thread-title {
    font-size: 14px;
  }

  .close-btn {
    background: none;
    border: none;
    color: #b5bac1;
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background: #35373c;
    color: #f2f3f5;
  }

  .thread-messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .loading,
  .error,
  .empty {
    color: #b5bac1;
    font-size: 13px;
    text-align: center;
    margin-top: 20px;
  }

  .error {
    color: #ed4245;
  }

  .thread-msg {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .msg-author {
    font-size: 13px;
    font-weight: 600;
    color: #f2f3f5;
  }

  .msg-time {
    font-size: 11px;
    color: #b5bac1;
  }

  .msg-content {
    font-size: 14px;
    color: #dbdee1;
    margin: 0;
    word-break: break-word;
  }

  .thread-input {
    padding: 12px;
    border-top: 1px solid #1e1f22;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .thread-input textarea {
    width: 100%;
    background: #383a40;
    border: none;
    border-radius: 6px;
    color: #dbdee1;
    padding: 8px 12px;
    font-size: 14px;
    resize: none;
    outline: none;
    font-family: inherit;
    box-sizing: border-box;
  }

  .thread-input textarea:focus {
    background: #404249;
  }

  .thread-input button {
    align-self: flex-end;
    background: #e5e7eb;
    border: none;
    border-radius: 4px;
    color: white;
    padding: 6px 16px;
    cursor: pointer;
    font-size: 13px;
  }

  .thread-input button:hover:not(:disabled) {
    background: #4752c4;
  }

  .thread-input button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
