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
	import { get } from 'svelte/store';
	import { channels } from '$lib/stores/channels';
	import { members } from '$lib/stores/members';
	import { roles } from '$lib/stores/roles';
	import LinkPreview from './LinkPreview.svelte';

	let { content, showPreview = true }: { content: string; showPreview?: boolean } = $props();

	// Extract the first plain URL from message content for link preview
	// Code blocks are stripped first so we don't preview URLs inside code
	const URL_RE = /https?:\/\/[^\s<>"'`]+[^\s<>"'`.,!?;:)\]]/g;
	const previewUrls = $derived.by(() => {
		if (!showPreview) return [];
		const stripped = content.replace(/```[\s\S]*?```/g, '').replace(/`[^`]+`/g, '');
		const matches = [...stripped.matchAll(URL_RE)].map(m => m[0]);
		return [...new Set(matches)].slice(0, 1);
	});

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


	function colorToHex(color: number | null): string {
		if (!color) return '#b5bac1';
		return '#' + color.toString(16).padStart(6, '0');
	}

	/** Replace Discord mention tokens with safe HTML before markdown parsing */
	function processMentions(text: string): string {
		const chList = get(channels);
		const memList = get(members);
		const roleList = get(roles);

		// <#channelId>
		text = text.replace(/<#(\d+)>/g, (_m: string, id: string) => {
			const ch = chList.find((c: {id:string}) => c.id === id);
			return '<span class="mention-chip mention-channel">#' + (ch ? (ch as any).name : id) + '</span>';
		});
		// <@userId> or <@!userId>
		text = text.replace(/<@!?(\d+)>/g, (_m: string, id: string) => {
			const mem = memList.find((m: any) => m.user_id === id || m.id === id);
			const name = mem ? ((mem as any).username ?? id) : id;
			return '<span class="mention-chip mention-user">@' + name + '</span>';
		});
		// <@&roleId>
		text = text.replace(/<@&(\d+)>/g, (_m: string, id: string) => {
			const role = roleList.find((r: {id:string}) => r.id === id);
			const hex = colorToHex(role ? (role as any).color : null);
			const name = role ? (role as any).name : id;
			return '<span class="mention-chip" style="color:' + hex + ';background:' + hex + '22;">@' + name + '</span>';
		});
		// @everyone / @here
		text = text.replace(/@(everyone|here)/g,
			'<span class="mention-chip mention-user">@$1</span>');
		// <t:timestamp:R> relative time
		text = text.replace(/<t:(\d+)(?::[A-Za-z])?>/g, (_m: string, ts: string) => {
			const ms = parseInt(ts) * 1000;
			const diff = Date.now() - ms;
			const abs = Math.abs(diff);
			const future = diff < 0;
			let label: string;
			if (abs < 60000) {
				label = 'just now';
			} else if (abs < 3600000) {
				const m = Math.round(abs / 60000);
				label = m + ' min' + (m !== 1 ? 's' : '') + (future ? ' from now' : ' ago');
			} else if (abs < 86400000) {
				const h = Math.round(abs / 3600000);
				label = h + ' hour' + (h !== 1 ? 's' : '') + (future ? ' from now' : ' ago');
			} else {
				const d = Math.round(abs / 86400000);
				label = d + ' day' + (d !== 1 ? 's' : '') + (future ? ' from now' : ' ago');
			}
			const iso = new Date(ms).toISOString();
			const loc = new Date(ms).toLocaleString();
			return '<time title="' + loc + '" datetime="' + iso + '">' + label + '</time>';
		});
		return text;
	}

	/**
	 * Sanitize HTML to prevent XSS while preserving code highlighting.
	 */
	function sanitize(html: string): string {
		html = html.replace(/<script[\s\S]*?<\/script>/gi, '').replace(/<style[\s\S]*?<\/style>/gi, '').replace(/<iframe[\s\S]*?<\/iframe>/gi, '');
		html = html.replace(/<(object|embed|form|meta|link|base|applet)[^>]*\/?>/gi, '');
		html = html.replace(/\s+on[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi, '');
		html = html.replace(/(href|src|action|formaction)\s*=\s*(['"]?)\s*(?:javascript|vbscript):[^'"\s>]*/gi, '$1=$2#');
		html = html.replace(/(href|src|action)\s*=\s*(['"]?)data:(?!image\/)[^'"\s>]*/gi, '$1=$2');
		return html;
	}

	onMount(() => {
		html = sanitize(marked.parse(processMentions(content), { renderer }) as string);
	});

	$effect(() => {
		html = sanitize(marked.parse(processMentions(content), { renderer }) as string);
	});
</script>

<div class="markdown-content">{@html html}</div>

{#each previewUrls as previewUrl (previewUrl)}
	<LinkPreview url={previewUrl} />
{/each}

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
		border-left: 4px solid #e5e7eb;
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
	.markdown-content :global(.mention-chip) {
		display: inline;
		padding: 0 3px;
		border-radius: 3px;
		font-weight: 500;
		cursor: default;
	}

	.markdown-content :global(.mention-user) {
		color: #c9cdfb;
		background: rgba(88, 101, 242, 0.1);
	}

	.markdown-content :global(.mention-channel) {
		color: #00aff4;
		background: rgba(0, 175, 244, 0.1);
	}

	.markdown-content :global(time) {
		color: #b5bac1;
		text-decoration: underline dotted;
		cursor: help;
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
