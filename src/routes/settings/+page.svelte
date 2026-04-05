<script lang="ts">
	import { seedColor, colorMode } from '$lib/stores/theme';
	import { nodeId, syncPeers, loadSyncState } from '$lib/stores/sync';
	import { locale, t } from '$lib/stores/i18n';
	import { preset } from '$lib/stores/preset';
	import { inventory } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { commands } from '$lib/tauri/commands';
	import type { AppPreset } from '$lib/tauri/types';

	let currentColor = $state('#34d399');
	let backupStatus = $state('');
	let confirmPreset = $state<AppPreset | null>(null);

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
			backupStatus = '…';
			const path = await commands.exportBackup();
			backupStatus = `${$t('backup_exported')} ${path}`;
		} catch (err) {
			backupStatus = `${$t('backup_error')} ${err}`;
		}
	}

	async function handleConfirmPreset() {
		if (!confirmPreset) return;
		await preset.switchTo(confirmPreset);
		await Promise.all([inventory.load(), categories.load()]);
		confirmPreset = null;
	}

	const presetColors = [
		'#34d399', '#6366f1', '#f43f5e', '#f59e0b',
		'#06b6d4', '#8b5cf6', '#ec4899', '#14b8a6',
	];

	const presets: { id: AppPreset; label: string; emoji: string }[] = [
		{ id: 'flowers',   label: $t('preset_flowers'),   emoji: '🌸' },
		{ id: 'ochokochi', label: $t('preset_ochokochi'), emoji: '🏢' },
		{ id: 'balanced',  label: $t('preset_balanced'),  emoji: '⚖️' },
	];
</script>

<div class="settings-page">
	<h1>{$t('page_settings_title')}</h1>

	<!-- ── Business Mode ──────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_business_mode')}</h2>
		<div class="preset-grid">
			{#each presets as p}
				<button
					class="preset-card"
					class:active={$preset === p.id}
					onclick={() => $preset !== p.id && (confirmPreset = p.id)}
				>
					<span class="preset-emoji">{p.emoji}</span>
					<span class="preset-label">{p.label}</span>
					{#if $preset === p.id}
						<span class="preset-active-dot"></span>
					{/if}
				</button>
			{/each}
		</div>

		{#if confirmPreset}
			{@const name = presets.find((p) => p.id === confirmPreset)?.label ?? confirmPreset}
			<div class="confirm-panel glass-sm">
				<p class="confirm-text">{$t('confirm_switch_preset', { name })}</p>
				<div class="confirm-actions">
					<button class="btn-primary" onclick={handleConfirmPreset}>{$t('action_confirm')}</button>
					<button class="btn-ghost" onclick={() => (confirmPreset = null)}>{$t('action_cancel')}</button>
				</div>
			</div>
		{/if}
	</section>

	<!-- ── Appearance ─────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_appearance')}</h2>

		<!-- Dark / Light toggle -->
		<div class="mode-toggle-row">
			<button
				class="mode-btn"
				class:mode-active={$colorMode === 'dark'}
				onclick={() => colorMode.set('dark')}
			>
				🌙 {$t('label_dark_mode')}
			</button>
			<button
				class="mode-btn"
				class:mode-active={$colorMode === 'light'}
				onclick={() => colorMode.set('light')}
			>
				☀️ {$t('label_light_mode')}
			</button>
		</div>
	</section>

	<!-- ── Theme Color ─────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_theme')}</h2>
		<p class="hint">{$t('hint_theme_color')}</p>

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
					aria-label="Цвет {color}"
				></button>
			{/each}
		</div>
	</section>

	<!-- ── Language ───────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_language')}</h2>
		<select bind:value={$locale} class="locale-select">
			<option value="ru">Русский</option>
			<option value="en">English</option>
		</select>
	</section>

	<!-- ── Sync & Identity ────────────────────────────────────── -->
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

	<!-- ── Backup ─────────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_backup')}</h2>
		<p class="hint">{$t('hint_backup_description')}</p>
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
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 12px;
		padding: 20px;
		margin-bottom: 16px;
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
	}

	/* ── Business preset cards ─── */
	.preset-grid {
		display: flex;
		gap: 10px;
		flex-wrap: wrap;
	}

	.preset-card {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 14px 20px;
		background: var(--glass-bg);
		border: 2px solid var(--glass-border);
		border-radius: 12px;
		cursor: pointer;
		transition: border-color 0.15s, background 0.15s;
		min-width: 100px;
		color: var(--color-on-surface);
	}

	.preset-card:hover {
		background: var(--glass-bg-hover);
		border-color: var(--color-outline);
	}

	.preset-card.active {
		border-color: var(--color-primary);
		background: rgba(52, 211, 153, 0.08);
	}

	.preset-emoji { font-size: 1.6rem; }

	.preset-label {
		font-size: 0.82rem;
		font-weight: 500;
	}

	.preset-active-dot {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-primary);
	}

	/* ── Confirm panel ────────── */
	.confirm-panel {
		margin-top: 12px;
		padding: 14px 16px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.confirm-text {
		font-size: 0.85rem;
		color: var(--color-on-surface);
		flex: 1;
	}

	.confirm-actions {
		display: flex;
		gap: 8px;
		flex-shrink: 0;
	}

	/* ── Appearance / mode toggle ── */
	.mode-toggle-row {
		display: flex;
		gap: 8px;
	}

	.mode-btn {
		padding: 8px 20px;
		border-radius: 10px;
		border: 1px solid var(--glass-border);
		background: var(--glass-bg);
		color: var(--color-on-surface);
		font-size: 0.85rem;
		cursor: pointer;
		transition: border-color 0.15s, background 0.15s;
	}

	.mode-btn:hover { background: var(--glass-bg-hover); }

	.mode-btn.mode-active {
		border-color: var(--color-primary);
		background: rgba(52, 211, 153, 0.10);
		color: var(--color-primary);
		font-weight: 600;
	}

	/* ── Color picker ─────────── */
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
		font-family: var(--font-mono);
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
		transition: border-color 0.15s, transform 0.1s;
	}

	.color-swatch.active { border-color: var(--color-on-surface); transform: scale(1.15); }
	.color-swatch:hover { border-color: var(--color-outline); }

	/* ── Language ─────────────── */
	.locale-select {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
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

	/* ── Sync identity ────────── */
	.info-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 0;
		border-bottom: 1px solid var(--glass-border);
	}

	.info-row:last-child { border-bottom: none; }

	.info-label {
		font-size: 0.85rem;
		color: var(--color-on-surface);
		opacity: 0.6;
	}

	.info-value {
		font-size: 0.85rem;
		font-family: var(--font-mono);
		color: var(--color-on-surface);
		word-break: break-all;
		text-align: right;
		max-width: 60%;
	}

	/* ── Buttons ──────────────── */
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

	.btn-primary:hover { opacity: 0.88; }

	.btn-ghost {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		padding: 8px 20px;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.875rem;
		transition: background 0.15s;
	}

	.btn-ghost:hover { background: var(--glass-bg-hover); }

	.backup-status {
		font-size: 0.8rem;
		color: var(--color-on-surface);
		opacity: 0.6;
		margin-top: 8px;
		word-break: break-all;
	}
</style>
