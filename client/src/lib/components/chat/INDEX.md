# /client/src/lib/components/chat/

Purpose: Chat UI components.

Pattern: Message display, input, reactions, rich content.

| Component              | Purpose                                                                          |
| ---------------------- | -------------------------------------------------------------------------------- |
| MessageList.svelte     | Scrollable list of messages with infinite scroll, reactions, inline emoji picker |
| MessageInput.svelte    | Text input with emoji picker, file upload, mentions, typing indicators           |
| MarkdownContent.svelte | Markdown renderer with syntax-highlighted code blocks (highlight.js)             |
| EmojiPicker.svelte     | Full emoji picker popup (emoji-mart) for reactions and message input             |
| MessageBubble.svelte   | Single message display with author, timestamp                                    |
| ReactionBar.svelte     | Emoji reactions on messages                                                      |
| EmbedRenderer.svelte   | Rich embeds (images, videos, URLs)                                               |
