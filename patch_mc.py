path = '/home/mb/opencorde/client/src/lib/components/chat/MarkdownContent.svelte'
with open(path) as f:
    c = f.read()

# 1. Add store imports
c = c.replace(
    "import { onMount } from 'svelte';",
    "import { onMount } from 'svelte';\n\timport { get } from 'svelte/store';\n\timport { channels } from '$lib/stores/channels';\n\timport { members } from '$lib/stores/members';\n\timport { roles } from '$lib/stores/roles';"
)

# 2. Add processMentions + colorToHex before sanitize
mention_fn = """
\tfunction colorToHex(color: number | null): string {
\t\tif (!color) return '#b5bac1';
\t\treturn '#' + color.toString(16).padStart(6, '0');
\t}

\t/** Replace Discord mention tokens with safe HTML before markdown parsing */
\tfunction processMentions(text: string): string {
\t\tconst chList = get(channels);
\t\tconst memList = get(members);
\t\tconst roleList = get(roles);

\t\t// <#channelId>
\t\ttext = text.replace(/<#(\\d+)>/g, (_m: string, id: string) => {
\t\t\tconst ch = chList.find((c: {id:string}) => c.id === id);
\t\t\treturn '<span class="mention-chip mention-channel">#' + (ch ? (ch as any).name : id) + '</span>';
\t\t});
\t\t// <@userId> or <@!userId>
\t\ttext = text.replace(/<@!?(\\d+)>/g, (_m: string, id: string) => {
\t\t\tconst mem = memList.find((m: any) => m.user_id === id || m.id === id);
\t\t\tconst name = mem ? ((mem as any).username ?? id) : id;
\t\t\treturn '<span class="mention-chip mention-user">@' + name + '</span>';
\t\t});
\t\t// <@&roleId>
\t\ttext = text.replace(/<@&(\\d+)>/g, (_m: string, id: string) => {
\t\t\tconst role = roleList.find((r: {id:string}) => r.id === id);
\t\t\tconst hex = colorToHex(role ? (role as any).color : null);
\t\t\tconst name = role ? (role as any).name : id;
\t\t\treturn '<span class="mention-chip" style="color:' + hex + ';background:' + hex + '22;">@' + name + '</span>';
\t\t});
\t\t// @everyone / @here
\t\ttext = text.replace(/@(everyone|here)/g,
\t\t\t'<span class="mention-chip mention-user">@$1</span>');
\t\t// <t:timestamp:R> relative time
\t\ttext = text.replace(/<t:(\\d+)(?::[A-Za-z])?>/g, (_m: string, ts: string) => {
\t\t\tconst ms = parseInt(ts) * 1000;
\t\t\tconst diff = Date.now() - ms;
\t\t\tconst abs = Math.abs(diff);
\t\t\tconst future = diff < 0;
\t\t\tlet label: string;
\t\t\tif (abs < 60000) {
\t\t\t\tlabel = 'just now';
\t\t\t} else if (abs < 3600000) {
\t\t\t\tconst m = Math.round(abs / 60000);
\t\t\t\tlabel = m + ' min' + (m !== 1 ? 's' : '') + (future ? ' from now' : ' ago');
\t\t\t} else if (abs < 86400000) {
\t\t\t\tconst h = Math.round(abs / 3600000);
\t\t\t\tlabel = h + ' hour' + (h !== 1 ? 's' : '') + (future ? ' from now' : ' ago');
\t\t\t} else {
\t\t\t\tconst d = Math.round(abs / 86400000);
\t\t\t\tlabel = d + ' day' + (d !== 1 ? 's' : '') + (future ? ' from now' : ' ago');
\t\t\t}
\t\t\tconst iso = new Date(ms).toISOString();
\t\t\tconst loc = new Date(ms).toLocaleString();
\t\t\treturn '<time title="' + loc + '" datetime="' + iso + '">' + label + '</time>';
\t\t});
\t\treturn text;
\t}

"""

c = c.replace(
    '\t/**\n\t * Sanitize HTML to prevent XSS',
    mention_fn + '\t/**\n\t * Sanitize HTML to prevent XSS'
)

# 3. Apply processMentions in both parse calls
c = c.replace(
    'html = sanitize(marked.parse(content, { renderer }) as string);',
    'html = sanitize(marked.parse(processMentions(content), { renderer }) as string);'
)

# 4. Add CSS
css_addition = """
\t.markdown-content :global(.mention-chip) {
\t\tdisplay: inline;
\t\tpadding: 0 3px;
\t\tborder-radius: 3px;
\t\tfont-weight: 500;
\t\tcursor: default;
\t}

\t.markdown-content :global(.mention-user) {
\t\tcolor: #c9cdfb;
\t\tbackground: rgba(88, 101, 242, 0.1);
\t}

\t.markdown-content :global(.mention-channel) {
\t\tcolor: #00aff4;
\t\tbackground: rgba(0, 175, 244, 0.1);
\t}

\t.markdown-content :global(time) {
\t\tcolor: #b5bac1;
\t\ttext-decoration: underline dotted;
\t\tcursor: help;
\t}"""

c = c.replace(
    '\t.markdown-content :global(a:hover) {\n\t\ttext-decoration: underline;\n\t}',
    '\t.markdown-content :global(a:hover) {\n\t\ttext-decoration: underline;\n\t}' + css_addition
)

with open(path, 'w') as f:
    f.write(c)

checks = ['processMentions' in c, 'mention-chip' in c, 'get(channels)' in c, '<t:' in c]
print('OK' if all(checks) else 'FAIL: ' + str({k: v for k, v in zip(['processMentions','mention-chip','channels','<t:'], checks)}))
