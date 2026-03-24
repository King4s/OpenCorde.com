<!--
  @file MarkdownContent.svelte
  @purpose Renders message content as markdown with syntax-highlighted code blocks
  @version 1.0.0
-->
<script lang="ts">
	import { marked } from 'marked';
	import hljs from 'highlight.js';
	import 'highlight.js/styles/atom-one-dark.css';
	import { onMount } from 'svelte';

	let { content }: { content: string } = $props();

	let html = $state('');

	// Configure marked with highlight.js for code blocks
	const renderer = new marked.Renderer();
	renderer.code = function({ text, lang }: { text: string; lang?: string }) {
		const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
		const highlighted = hljs.highlight(text, { language }).value;
		return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
	};

	// Inline code renderer for backticks
	renderer.codespan = function({ text }: { text: string }) {
		return `<code class="inline-code">${text}</code>`;
	};

	// Configure marked options
	marked.setOptions({
		breaks: true,
		gfm: true,
	});

	/**
	 * Sanitize HTML to prevent XSS while preserving code highlighting.
	 * Removes script tags, iframes, and event handlers.
	 */
	function sanitize(html: string): string {
		return html
			.replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '')
			.replace(/<iframe[^>]*>[\s\S]*?<\/iframe>/gi, '')
			.replace(/on\w+="[^"]*"/gi, '')
			.replace(/on\w+='[^']*'/gi, '');
	}

	onMount(() => {
		html = sanitize(marked.parse(content, { renderer }) as string);
	});

	$effect(() => {
		html = sanitize(marked.parse(content, { renderer }) as string);
	});
</script>

<div class="markdown-content">{@html html}</div>

<style>
	:global(.hljs) {
		background: #1e1f22;
		border-radius: 6px;
		padding: 12px 16px;
		overflow-x: auto;
		font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
		font-size: 13px;
		line-height: 1.5;
		border: 1px solid #35373c;
	}

	:global(.hljs-keyword) {
		color: #cc99cd;
	}

	:global(.hljs-string) {
		color: #7ec699;
	}

	:global(.hljs-comment) {
		color: #999;
		font-style: italic;
	}

	:global(.hljs-number) {
		color: #f08d49;
	}

	:global(.hljs-function) {
		color: #6fb3d2;
	}

	:global(.hljs-built_in) {
		color: #6fb3d2;
	}

	:global(.hljs-variable) {
		color: #e8bf6a;
	}

	:global(.hljs-type) {
		color: #6fb3d2;
	}

	:global(.hljs-attr) {
		color: #9cdcfe;
	}

	:global(.hljs-literal) {
		color: #cc99cd;
	}

	:global(.hljs-punctuation) {
		color: #dbdee1;
	}

	.markdown-content {
		color: #dbdee1;
		font-size: 15px;
		line-height: 1.375;
		word-break: break-word;
	}

	.markdown-content :global(p) {
		margin: 0 0 4px 0;
	}

	.markdown-content :global(p:last-child) {
		margin-bottom: 0;
	}

	.markdown-content :global(strong) {
		color: #f2f3f5;
		font-weight: 700;
	}

	.markdown-content :global(em) {
		font-style: italic;
	}

	.markdown-content :global(.inline-code) {
		background: #1e1f22;
		border-radius: 3px;
		padding: 2px 4px;
		font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
		font-size: 13px;
		color: #e3c4a8;
		border: 1px solid #35373c;
	}

	.markdown-content :global(pre) {
		margin: 4px 0;
	}

	.markdown-content :global(blockquote) {
		border-left: 4px solid #5865f2;
		padding: 4px 12px;
		margin: 4px 0;
		color: #b5bac1;
		background: #1e1f22;
		border-radius: 0 4px 4px 0;
	}

	.markdown-content :global(a) {
		color: #00aff4;
		text-decoration: none;
	}

	.markdown-content :global(a:hover) {
		text-decoration: underline;
	}

	.markdown-content :global(h1),
	.markdown-content :global(h2),
	.markdown-content :global(h3) {
		color: #f2f3f5;
		margin: 8px 0 4px 0;
	}

	.markdown-content :global(ul),
	.markdown-content :global(ol) {
		padding-left: 20px;
		margin: 4px 0;
	}

	.markdown-content :global(li) {
		margin: 2px 0;
	}

	.markdown-content :global(code) {
		white-space: pre-wrap;
		word-break: break-word;
	}

	.markdown-content :global(pre code) {
		background: none;
		padding: 0;
		border: none;
		color: inherit;
	}
</style>
