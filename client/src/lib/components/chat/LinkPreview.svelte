<!--
  @file LinkPreview.svelte
  @purpose Renders an OpenGraph / link preview card below a message that contains a URL.
  @version 1.0.0
-->
<script lang="ts">
	import api from '$lib/api/client';

	interface UnfurlData {
		url: string;
		title: string | null;
		description: string | null;
		image_url: string | null;
		site_name: string | null;
	}

	let { url }: { url: string } = $props();

	let data = $state<UnfurlData | null>(null);
	let failed = $state(false);

	$effect(() => {
		if (!url) return;
		failed = false;
		data = null;

		api.get<UnfurlData>(`/unfurl?url=${encodeURIComponent(url)}`)
			.then((d) => {
				// Only show card if there's at least a title
				if (d.title) {
					data = d;
				} else {
					failed = true;
				}
			})
			.catch(() => {
				failed = true;
			});
	});

	function truncate(s: string, max: number): string {
		return s.length > max ? s.slice(0, max) + '…' : s;
	}
</script>

{#if data}
	<a
		href={data.url}
		target="_blank"
		rel="noopener noreferrer"
		class="preview-card"
		aria-label="Link preview: {data.title}"
	>
		{#if data.image_url}
			<div class="preview-image">
				<img src={data.image_url} alt="" loading="lazy" onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none'; }} />
			</div>
		{/if}
		<div class="preview-body">
			{#if data.site_name}
				<p class="preview-site">{data.site_name}</p>
			{/if}
			<p class="preview-title">{truncate(data.title!, 120)}</p>
			{#if data.description}
				<p class="preview-desc">{truncate(data.description, 200)}</p>
			{/if}
		</div>
	</a>
{/if}

<style>
	.preview-card {
		display: flex;
		gap: 12px;
		margin-top: 8px;
		padding: 12px;
		background: #2b2d31;
		border: 1px solid #35373c;
		border-left: 4px solid #e5e7eb;
		border-radius: 4px;
		text-decoration: none;
		max-width: 480px;
		overflow: hidden;
		transition: background 0.1s;
	}
	.preview-card:hover {
		background: #313338;
	}
	.preview-image {
		flex-shrink: 0;
		width: 80px;
		height: 80px;
		border-radius: 4px;
		overflow: hidden;
		background: #1e1f22;
	}
	.preview-image img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.preview-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.preview-site {
		margin: 0;
		font-size: 11px;
		color: #b5bac1;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.preview-title {
		margin: 0;
		font-size: 14px;
		font-weight: 600;
		color: #00aff4;
		line-height: 1.3;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
	}
	.preview-desc {
		margin: 2px 0 0 0;
		font-size: 12px;
		color: #b5bac1;
		line-height: 1.4;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
	}
</style>
