/**
 * @file Markdown renderer — lightweight Discord-style markdown
 * @purpose Convert message content to safe HTML without external dependencies
 */

/**
 * Escape HTML special characters to prevent XSS attacks.
 * Applied after code blocks are extracted to protect remaining content.
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/**
 * Render Discord-style markdown to safe HTML.
 * Supports: **bold**, *italic*, __underline__, ~~strikethrough~~,
 * `inline code`, ```code blocks```, > blockquotes, spoilers ||text||
 *
 * Strategy: Extract code blocks first (unescaped), escape remaining HTML,
 * apply markdown transforms, then re-insert code blocks.
 */
export function renderMarkdown(text: string): string {
  // Step 1: Extract code blocks BEFORE escaping (we'll re-insert after)
  const codeBlocks: string[] = [];
  let processed = text.replace(
    /```(?:(\w+)\n)?([\s\S]*?)```/g,
    (_, lang, code) => {
      const escaped = escapeHtml(code.trimEnd());
      const langAttr = lang ? ` class="language-${escapeHtml(lang)}"` : "";
      codeBlocks.push(
        `<pre class="bg-gray-900 rounded px-3 py-2 my-1 overflow-x-auto text-xs font-mono"><code${langAttr}>${escaped}</code></pre>`,
      );
      return `\x00CODE${codeBlocks.length - 1}\x00`;
    },
  );

  // Step 2: Extract inline code (backticks)
  const inlineCodes: string[] = [];
  processed = processed.replace(/`([^`\n]+?)`/g, (_, code) => {
    inlineCodes.push(
      `<code class="bg-gray-900 rounded px-1 py-0.5 text-xs font-mono text-gray-300">${escapeHtml(code)}</code>`,
    );
    return `\x00INLINE${inlineCodes.length - 1}\x00`;
  });

  // Step 3: Escape remaining HTML (safe now that code is extracted)
  processed = escapeHtml(processed);

  // Step 4: Apply markdown transformations (order matters for correct precedence)
  // Bold **text** or __text__
  processed = processed.replace(/\*\*(.+?)\*\*/gs, "<strong>$1</strong>");
  processed = processed.replace(/__(.+?)__/gs, "<strong>$1</strong>");

  // Italic *text* or _text_ (not inside words, use negative lookbehind/lookahead)
  processed = processed.replace(
    /(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)/gs,
    "<em>$1</em>",
  );
  processed = processed.replace(
    /(?<!_)_(?!_)(.+?)(?<!_)_(?!_)/gs,
    "<em>$1</em>",
  );

  // Strikethrough ~~text~~
  processed = processed.replace(/~~(.+?)~~/gs, "<s>$1</s>");

  // Spoiler ||text|| (hidden until hover)
  processed = processed.replace(
    /\|\|(.+?)\|\|/gs,
    '<span class="bg-gray-700 text-gray-700 hover:text-white rounded cursor-pointer select-none transition-colors px-0.5">$1</span>',
  );

  // Blockquotes (> at start of line — note > was escaped to &gt;)
  processed = processed.replace(
    /^&gt; (.+)$/gm,
    '<div class="border-l-4 border-gray-500 pl-2 my-1 text-gray-400">$1</div>',
  );

  // Line breaks
  processed = processed.replace(/\n/g, "<br>");

  // Step 5: Re-insert code blocks and inline code in correct order
  processed = processed.replace(
    /\x00CODE(\d+)\x00/g,
    (_, i) => codeBlocks[parseInt(i)],
  );
  processed = processed.replace(
    /\x00INLINE(\d+)\x00/g,
    (_, i) => inlineCodes[parseInt(i)],
  );

  return processed;
}
