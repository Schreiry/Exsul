<script lang="ts">
	import { seedColor } from '$lib/stores/theme';
	import { nodeId, syncPeers, loadSyncState } from '$lib/stores/sync';
	import { locale, t } from '$lib/stores/i18n';
	import { commands } from '$lib/tauri/commands';

	let currentColor = $state('#34d399');
	let backupStatus = $state('');

	$effect(() => {
		return seedColor.subscribe((c) => (currentColor = c));
	});

	$effect(() => {
		loadSyncState();
	});

	function handleColorChange(e: Event) {
		const input = e.target as HTMLInputElement;
		seedColor.set(input.value);
	}

	async function handleExportBackup() {
		try {
			backupStatus = 'Creating backup...';
			const path = await commands.exportBackup();
			backupStatus = `${$t('backup_exported')} ${path}`;
		} catch (err) {
			backupStatus = `${$t('backup_error')} ${err}`;
		}
	}

	const presetColors = [
		'#34d399', '#6366f1', '#f43f5e', '#f59e0b',
		'#06b6d4', '#8b5cf6', '#ec4899', '#14b8a6',
	];
</script>

<div class="settings-page">
	<h1>{$t('page_settings_title')}</h1>

	<section class="section">
		<h2>{$t('label_theme')}</h2>
		<p class="hint">Pick a seed color to generate your entire UI palette.</p>

		<div class="color-picker-row">
			<input type="color" value={currentColor} oninput={handleColorChange} class="color-input" />
			<span class="color-hex">{currentColor}</span>
		</div>

		<div class="preset-colors">
			{#each presetColors as color}
				<button
					class="color-swatch"
					style:background={color}
					class:active={currentColor === color}
					onclick={() => seedColor.set(color)}
					aria-label="Set color to {color}"
				></button>
			{/each}
		</div>
	</section>

	<section class="section">
		<h2>{$t('label_language')}</h2>
		<select bind:value={$locale} class="locale-select">
			<option value="en">English</option>
			<option value="ru">Русский</option>
		</select>
	</section>

	<section class="section">
		<h2>{$t('sync_identity')}</h2>
		<div class="info-row">
			<span class="info-label">{$t('label_node_id')}</span>
			<code class="info-value">{$nodeId || '...'}</code>
		</div>
		<div class="info-row">
			<span class="info-label">{$t('label_peers')}</span>
			<span class="info-value">{$syncPeers.length}</span>
		</div>
	</section>

	<section class="section">
		<h2>{$t('label_backup')}</h2>
		<p class="hint">Backups are AES-256-GCM encrypted and created automatically on app close.</p>
		<button class="btn-primary" onclick={handleExportBackup}>
			{$t('action_export_backup')}
		</button>
		{#if backupStatus}
			<p class="backup-status">{backupStatus}</p>
		{/if}
	</section>
</div>

<style>
	.settings-page { max-width: 600px; margin: 0 auto; }

	h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 24px;
	}

	h2 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 12px;
	}

	.hint {
		font-size: 0.8rem;
		color: var(--color-on-surface);
		opacity: 0.5;
		margin-bottom: 16px;
	}

	.section {
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
		padding: 20px;
		margin-bottom: 16px;
	}

	.color-picker-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 16px;
	}

	.color-input {
		width: 48px;
		height: 48px;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		background: transparent;
	}

	.color-input::-webkit-color-swatch-wrapper { padding: 0; }

	.color-input::-webkit-color-swatch {
		border: 2px solid var(--color-outline-variant);
		border-radius: 8px;
	}

	.color-hex {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: var(--color-on-surface);
		opacity: 0.6;
	}

	.preset-colors { display: flex; gap: 8px; flex-wrap: wrap; }

	.color-swatch {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.color-swatch.active { border-color: var(--color-on-surface); }
	.color-swatch:hover { border-color: var(--color-outline-variant); }

	.locale-select {
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 8px 14px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		cursor: pointer;
		outline: none;
		min-width: 160px;
	}

	.locale-select:focus { border-color: var(--color-primary); }

	.info-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 0;
		border-bottom: 1px solid var(--color-outline-variant);
	}

	.info-row:last-child { border-bottom: none; }

	.info-label {
		font-size: 0.85rem;
		color: var(--color-on-surface);
		opacity: 0.6;
	}

	.info-value {
		font-size: 0.85rem;
		font-family: 'Courier New', monospace;
		color: var(--color-on-surface);
	}

	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary);
		border: none;
		padding: 8px 20px;
		border-radius: 8px;
		font-weight: 600;
		cursor: pointer;
		font-size: 0.875rem;
		transition: opacity 0.15s;
	}

	.btn-primary:hover { opacity: 0.9; }

	.backup-status {
		font-size: 0.8rem;
		color: var(--color-on-surface);
		opacity: 0.6;
		margin-top: 8px;
		word-break: break-all;
	}
</style>
