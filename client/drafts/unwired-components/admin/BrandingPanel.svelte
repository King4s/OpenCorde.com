<script lang="ts">
	/**
	 * @file Instance Branding Panel
	 * @purpose Admin controls: instance name, tagline, color, logo, favicon, custom CSS
	 */
	import api from '$lib/api/client';
	import type { InstanceBranding, BrandingImageUploadResponse } from '$lib/api/types';

	let branding = $state<InstanceBranding | null>(null);
	let loading = $state(false);
	let saving = $state(false);
	let error = $state('');
	let successMsg = $state('');

	// Editable form fields
	let instanceName = $state('');
	let tagline = $state('');
	let primaryColor = $state('#5865F2');
	let customCss = $state('');
	let baseUrl = $state('');

	// Logo upload state
	let logoUploading = $state(false);
	let logoUrl = $state<string | null>(null);

	// Favicon upload state
	let faviconUploading = $state(false);
	let faviconUrl = $state<string | null>(null);

	// Preview mode
	let showPreview = $state(false);

	$effect(() => {
		loadBranding();
	});

	async function loadBranding() {
		try {
			loading = true;
			branding = await api.get<InstanceBranding>('/admin/branding');
			instanceName = branding.instance_name ?? '';
			tagline = branding.tagline ?? '';
			primaryColor = branding.primary_color ?? '#5865F2';
			customCss = branding.custom_css ?? '';
			baseUrl = branding.base_url ?? '';
			logoUrl = branding.logo_url;
			faviconUrl = branding.favicon_url;
			error = '';
		} catch (e: any) {
			error = e.message?.includes('403') ? 'Access denied. Admin privileges required.' : (e.message ?? 'Failed to load branding');
			branding = null;
		} finally { loading = false; }
	}

	async function saveBranding() {
		saving = true;
		error = '';
		successMsg = '';
		try {
			branding = await api.put<InstanceBranding>('/admin/branding', {
				instance_name: instanceName || null,
				tagline: tagline || null,
				primary_color: primaryColor || null,
				logo_url: logoUrl,
				favicon_url: faviconUrl,
				custom_css: customCss || null,
				base_url: baseUrl || null,
			});
			successMsg = 'Branding settings saved.';
			setTimeout(() => successMsg = '', 3000);
		} catch (e: any) {
			error = e.message ?? 'Failed to save branding';
		} finally { saving = false; }
	}

	async function handleRemoveLogo() {
		logoUrl = null;
		error = '';
		try {
			await api.put<InstanceBranding>('/admin/branding', { logo_url: null });
			successMsg = 'Logo removed.';
			setTimeout(() => successMsg = '', 3000);
		} catch (e: any) {
			error = e.message ?? 'Failed to remove logo';
		}
	}

	async function handleRemoveFavicon() {
		faviconUrl = null;
		error = '';
		try {
			await api.put<InstanceBranding>('/admin/branding', { favicon_url: null });
			successMsg = 'Favicon removed.';
			setTimeout(() => successMsg = '', 3000);
		} catch (e: any) {
			error = e.message ?? 'Failed to remove favicon';
		}
	}

	async function handleLogoUpload() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = 'image/png,image/jpeg,image/webp,image/svg+xml';
		input.onchange = async () => {
			const file = input.files?.[0];
			if (!file) return;

			logoUploading = true;
			error = '';
			try {
				const formData = new FormData();
				formData.append('file', file);
				const result = await api.postFormData<BrandingImageUploadResponse>('/admin/branding/logo', formData);
				logoUrl = result.url;
				successMsg = 'Logo uploaded successfully.';
				setTimeout(() => successMsg = '', 3000);
			} catch (e: any) {
				error = e.message ?? 'Failed to upload logo';
			} finally { logoUploading = false; }
		};
		input.click();
	}

	async function handleFaviconUpload() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = 'image/png,image/jpeg,image/webp,image/svg+xml,image/x-icon';
		input.onchange = async () => {
			const file = input.files?.[0];
			if (!file) return;

			faviconUploading = true;
			error = '';
			try {
				const formData = new FormData();
				formData.append('file', file);
				const result = await api.postFormData<BrandingImageUploadResponse>('/admin/branding/favicon', formData);
				faviconUrl = result.url;
				successMsg = 'Favicon uploaded successfully.';
				setTimeout(() => successMsg = '', 3000);
			} catch (e: any) {
				error = e.message ?? 'Failed to upload favicon';
			} finally { faviconUploading = false; }
		};
		input.click();
	}

	async function resetBranding() {
		const confirmed = confirm(
			'Reset all branding to defaults?\n\nThis will clear the tagline, primary color, logo, favicon, and custom CSS. The instance name and base URL will be kept.'
		);
		if (!confirmed) return;

		saving = true;
		error = '';
		successMsg = '';
		try {
			branding = await api.post<InstanceBranding>('/admin/branding/reset');
			instanceName = branding.instance_name ?? '';
			tagline = branding.tagline ?? '';
			primaryColor = branding.primary_color ?? '#5865F2';
			customCss = branding.custom_css ?? '';
			baseUrl = branding.base_url ?? '';
			logoUrl = null;
			faviconUrl = null;
			successMsg = 'Branding reset to defaults.';
			setTimeout(() => successMsg = '', 3000);
		} catch (e: any) {
			error = e.message ?? 'Failed to reset branding';
		} finally { saving = false; }
	}
</script>

<div class="bg-gray-800 rounded-lg p-4 sm:p-6 space-y-6">
	<!-- Error / Success messages -->
	{#if error}
		<div class="px-4 py-3 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm">{error}</div>
	{/if}
	{#if successMsg}
		<div class="px-4 py-3 bg-green-900/20 border border-green-800/40 rounded text-green-300 text-sm">{successMsg}</div>
	{/if}

	{#if loading}
		<div class="text-gray-400 text-sm py-8 text-center">Loading branding settings...</div>
	{:else}
		<!-- Instance Identity -->
		<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
			<h3 class="text-white font-medium text-lg mb-4">Instance Identity</h3>

			<div class="space-y-4">
				<label class="block">
					<span class="text-gray-300 text-sm">Instance Name</span>
					<input
						type="text"
						bind:value={instanceName}
						maxlength="128"
						placeholder="My OpenCorde Instance"
						class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-gray-500"
					/>
					<span class="text-gray-500 text-xs mt-1 block">Shown in login page header and browser title.</span>
				</label>

				<label class="block">
					<span class="text-gray-300 text-sm">Tagline</span>
					<input
						type="text"
						bind:value={tagline}
						maxlength="256"
						placeholder="Your community, your way."
						class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-gray-500"
					/>
					<span class="text-gray-500 text-xs mt-1 block">A short description shown below the instance name.</span>
				</label>

				<label class="block">
					<span class="text-gray-300 text-sm">Base URL</span>
					<input
						type="text"
						bind:value={baseUrl}
						maxlength="256"
						placeholder="https://chat.example.com"
						class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-gray-500"
					/>
					<span class="text-gray-500 text-xs mt-1 block">Public URL of this instance.</span>
				</label>
			</div>
		</div>

		<!-- Appearance -->
		<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
			<h3 class="text-white font-medium text-lg mb-4">Appearance</h3>

			<div class="space-y-4">
				<label class="block">
					<span class="text-gray-300 text-sm">Primary Color</span>
					<div class="flex items-center gap-3 mt-1">
						<input
							type="color"
							bind:value={primaryColor}
							class="w-10 h-10 rounded cursor-pointer border border-gray-600 bg-transparent p-0.5"
						/>
						<input
							type="text"
							bind:value={primaryColor}
							maxlength="32"
							placeholder="#5865F2"
							class="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm font-mono focus:outline-none focus:border-gray-500 w-32"
						/>
						<button
							onclick={() => primaryColor = '#5865F2'}
							class="px-2 py-1 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors"
							type="button"
						>Default</button>
					</div>
					<span class="text-gray-500 text-xs mt-1 block">Used for buttons, links, and accent elements throughout the app.</span>
				</label>

				<!-- Logo upload -->
				<div class="pt-2">
					<span class="text-gray-300 text-sm block mb-2">Logo</span>
					<div class="flex items-start gap-4">
						{#if logoUrl}
							<div class="w-32 h-10 bg-gray-900/60 rounded flex items-center justify-center overflow-hidden border border-gray-600">
								<img src={logoUrl} alt="Instance logo" class="max-w-full max-h-full object-contain" />
							</div>
						{:else}
							<div class="w-32 h-10 bg-gray-900/60 rounded flex items-center justify-center border border-gray-600">
								<span class="text-gray-500 text-xs">No logo</span>
							</div>
						{/if}
						<div class="flex flex-col gap-2">
							<button
								onclick={handleLogoUpload}
								disabled={logoUploading}
								class="px-4 py-2 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-600 text-white rounded text-sm font-medium transition-colors"
								type="button"
							>{logoUploading ? 'Uploading...' : logoUrl ? 'Replace Logo' : 'Upload Logo'}</button>
							{#if logoUrl}
								<button
									onclick={handleRemoveLogo}
									class="px-4 py-1 text-xs rounded text-red-300 hover:text-red-200 transition-colors"
									type="button"
								>Remove</button>
							{/if}
						</div>
					</div>
					<span class="text-gray-500 text-xs mt-1 block">Recommended: 200×64px, PNG or SVG. Max 2 MB.</span>
				</div>

				<!-- Favicon upload -->
				<div class="pt-2">
					<span class="text-gray-300 text-sm block mb-2">Favicon</span>
					<div class="flex items-start gap-4">
						{#if faviconUrl}
							<div class="w-8 h-8 bg-gray-900/60 rounded flex items-center justify-center overflow-hidden border border-gray-600">
								<img src={faviconUrl} alt="Favicon" class="w-full h-full object-contain" />
							</div>
						{:else}
							<div class="w-8 h-8 bg-gray-700 rounded flex items-center justify-center border border-gray-600">
								<span class="text-gray-500 text-[10px]">--</span>
							</div>
						{/if}
						<div class="flex flex-col gap-2">
							<button
								onclick={handleFaviconUpload}
								disabled={faviconUploading}
								class="px-4 py-2 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-600 text-white rounded text-sm font-medium transition-colors"
								type="button"
							>{faviconUploading ? 'Uploading...' : faviconUrl ? 'Replace Favicon' : 'Upload Favicon'}</button>
							{#if faviconUrl}
								<button
									onclick={handleRemoveFavicon}
									class="px-4 py-1 text-xs rounded text-red-300 hover:text-red-200 transition-colors"
									type="button"
								>Remove</button>
							{/if}
						</div>
					</div>
					<span class="text-gray-500 text-xs mt-1 block">Shown in browser tabs. Recommended: 32×32px, PNG or ICO. Max 2 MB.</span>
				</div>
			</div>
		</div>

		<!-- Custom CSS -->
		<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
			<h3 class="text-white font-medium text-lg mb-4">Custom CSS</h3>
			<div class="space-y-4">
				<label class="block">
					<span class="text-gray-300 text-sm">Custom CSS</span>
					<textarea
						bind:value={customCss}
						maxlength="65536"
						rows="8"
						placeholder="/* Add custom CSS here */"
						class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white text-sm font-mono focus:outline-none focus:border-gray-500 resize-y"
					></textarea>
					<span class="text-gray-500 text-xs mt-1 block">CSS rules applied globally. Max 64 KB. Use <code class="bg-gray-700 px-1 rounded">:root</code> variables for theming.</span>
				</label>
			</div>
		</div>

		<!-- Preview -->
		<div class="border border-gray-700 rounded-lg p-4 sm:p-6">
			<div class="flex flex-wrap items-center justify-between gap-3 mb-4">
				<h3 class="text-white font-medium text-lg">Preview</h3>
				<button
					onclick={() => showPreview = !showPreview}
					class="px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded text-sm font-medium transition-colors"
					type="button"
				>
					{showPreview ? 'Hide Preview' : 'Show Preview'}
				</button>
			</div>

			{#if showPreview}
				<div class="border border-gray-600 rounded-lg overflow-hidden" style="background-color: #1a1a2e;">
					<!-- Preview header bar -->
					<div class="flex items-center gap-3 px-4 py-3 border-b border-gray-700" style="background-color: #16162a;">
						{#if logoUrl}
							<img src={logoUrl} alt="Logo" class="h-6 object-contain" />
						{/if}
						<span class="text-white text-sm font-medium">{instanceName || 'OpenCorde'}</span>
						<span class="text-gray-500 text-xs">|</span>
						<span class="text-gray-400 text-xs">{tagline || 'Connect freely'}</span>
					</div>
					<!-- Preview body -->
					<div class="p-6 space-y-4">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 rounded-full flex items-center justify-center text-white text-sm font-bold" style="background-color: {primaryColor};">{instanceName ? instanceName.charAt(0).toUpperCase() : 'O'}</div>
							<div>
								<div class="text-white text-sm font-medium">{instanceName || 'OpenCorde'}</div>
								<div class="text-gray-500 text-xs">{tagline || 'Your community, your way.'}</div>
							</div>
						</div>
						<div>
							<button
								class="px-4 py-2 rounded text-white text-sm font-medium transition-colors"
								style="background-color: {primaryColor};"
							>Primary Button</button>
						</div>
						<div class="flex gap-2">
							<div class="px-3 py-1 rounded text-xs text-white" style="background-color: {primaryColor}; opacity: 0.2;">
								<span style="color: {primaryColor};">Accent badge</span>
							</div>
						</div>
						<div class="h-px" style="background-color: {primaryColor}; opacity: 0.3;"></div>
						<div class="text-gray-400 text-xs">
							The primary color and branding elements appear on the login page, in the app header, and in notification emails.
						</div>
					</div>
				</div>
			{:else}
				<div class="text-gray-500 text-sm">Click "Show Preview" to see how your branding changes will look.</div>
			{/if}
		</div>

		<!-- Actions -->
		<div class="flex flex-wrap items-center gap-3 pt-2">
			<button
				onclick={saveBranding}
				disabled={saving}
				class="px-6 py-2.5 bg-gray-600 hover:bg-gray-500 disabled:bg-gray-600 text-white rounded text-sm font-medium transition-colors"
			>{saving ? 'Saving...' : 'Save Branding'}</button>
			<button
				onclick={resetBranding}
				disabled={saving}
				class="px-4 py-2.5 bg-transparent border border-red-800/40 hover:border-red-700/60 text-red-300 rounded text-sm font-medium transition-colors"
			>Reset to Defaults</button>
		</div>
	{/if}
</div>
