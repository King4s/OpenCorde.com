/**
 * @file Mention utilities for message input
 * @purpose Query detection, token formatting, and best-effort text→mention resolution
 * @version 1.0.0
 */
export type MentionType = 'user' | 'role' | 'channel' | 'special';

export interface MentionOption {
	type: MentionType;
	id: string;
	name: string;
	displayName?: string;
	color?: number | null;
	avatarUrl?: string | null;
	mentionable?: boolean;
}

/**
 * Search backward from cursor for an unclosed '@' mention query.
 * Returns the query text and the index where '@' starts, or null if no active query.
 */
export function findMentionQuery(text: string, cursorPos: number): { query: string; startIndex: number } | null {
	let start = -1;
	for (let i = cursorPos - 1; i >= 0; i--) {
		if (text[i] === '@') {
			start = i;
			break;
		}
		if (text[i] === ' ' || text[i] === '\n' || text[i] === '\t') {
			return null;
		}
	}
	if (start === -1) return null;
	// Must be preceded by start-of-string or whitespace/punctuation
	const prevChar = start > 0 ? text[start - 1] : '';
	if (prevChar && !/\s|[`()\[\]{}<>"']/.test(prevChar)) {
		return null;
	}
	const query = text.slice(start + 1, cursorPos);
	// Query must not contain '@' (that would mean a new mention started)
	if (query.includes('@')) return null;
	return { query, startIndex: start };
}

/**
 * Build the raw token string for a mention option.
 */
export function buildMentionToken(option: MentionOption): string {
	switch (option.type) {
		case 'user':
			return `<@${option.id}>`;
		case 'role':
			return `<@&${option.id}>`;
		case 'channel':
			return `<#${option.id}>`;
		case 'special':
			return `@${option.name}`;
		default:
			return `@${option.name}`;
	}
}

/**
 * Best-effort resolution of plain @name text into mention tokens.
 * Only replaces whole-word matches bounded by whitespace or punctuation.
 * Skips tokens that are already in Discord format (<@id> etc).
 */
export function resolveTextMentions(
	text: string,
	members: { id: string; username: string; nickname?: string | null }[],
	roles: { id: string; name: string; mentionable?: boolean }[],
	channels: { id: string; name: string }[],
): string {
	// Build a lookup map: lowercase name → token
	const map = new Map<string, string>();

	for (const m of members) {
		const names = [m.username, m.nickname].filter(Boolean) as string[];
		for (const n of names) {
			map.set(n.toLowerCase(), `<@${m.id}>`);
		}
	}
	for (const r of roles) {
		map.set(r.name.toLowerCase(), `<@&${r.id}>`);
	}
	for (const c of channels) {
		map.set(c.name.toLowerCase(), `<#${c.id}>`);
	}
	// Special mentions
	map.set('everyone', '@everyone');
	map.set('here', '@here');

	// Replace whole-word @name occurrences, skipping already-tokenized mentions
	// Regex: @ followed by word chars, not preceded by < (to avoid <@...)
	return text.replace(/(?<!<)@(\w+)/g, (match, name) => {
		const token = map.get(name.toLowerCase());
		return token ?? match;
	});
}
