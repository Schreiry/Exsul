<script lang="ts">
	import { seedColor, colorMode, uiScale, paletteMode } from '$lib/stores/theme';
	import { nodeId, syncPeers, loadSyncState } from '$lib/stores/sync';
	import { locale, t } from '$lib/stores/i18n';
	import { preset } from '$lib/stores/preset';
	import { inventory } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { flowerConstants } from '$lib/stores/flowers';
	import { commands } from '$lib/tauri/commands';
	import { globalCurrency, CURRENCIES } from '$lib/stores/currency';
	import GlassDropdown from '$lib/components/common/GlassDropdown.svelte';
	import type { AppPreset, PricingMode } from '$lib/tauri/types';

	let currentColor = $state('#34d399');
	let backupStatus = $state('');
	let confirmPreset = $state<AppPreset | null>(null);

	// Flower constants draft (only when preset=flowers)
	let constantsDraft = $state({
		weight_per_flower: 0.05,
		flowers_per_pack: 10,
		price_per_pack: 500,
		price_per_flower: 50,
		pricing_mode: 'pack' as PricingMode,
	});
	let constantsSaving = $state(false);

	$effect(() => {
		if ($preset === 'flowers') {
			flowerConstants.load();
		}
	});

	$effect(() => {
		const c = $flowerConstants;
		if (c) {
			constantsDraft = {
				weight_per_flower: c.weight_per_flower,
				flowers_per_pack: c.flowers_per_pack,
				price_per_pack: c.price_per_pack,
				price_per_flower: c.price_per_flower,
				pricing_mode: (c.pricing_mode ?? 'pack') as PricingMode,
			};
		}
	});

	async function handleSaveConstants() {
		constantsSaving = true;
		try {
			await flowerConstants.save(constantsDraft);
		} finally {
			constantsSaving = false;
		}
	}

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

	let importBusy = $state(false);

	async function handleExportBackup() {
		try {
			backupStatus = '…';
			await commands.exportBackup();
			backupStatus = '✓ Бэкап создан';
		} catch (err) {
			backupStatus = `${$t('backup_error')} ${err}`;
		}
	}

	async function handleImportBackup() {
		if (!confirm('Восстановить данные из резервной копии? Текущие данные будут заменены.')) return;
		// Use native <input type="file"> — no extra plugin needed
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = '.bak';
		input.onchange = async () => {
			const file = input.files?.[0];
			if (!file) return;
			importBusy = true;
			backupStatus = '…';
			try {
				const buffer = await file.arrayBuffer();
				const bytes = Array.from(new Uint8Array(buffer));
				await commands.importBackupData(bytes);
				backupStatus = '✓ Данные восстановлены. Перезапустите приложение.';
			} catch (err) {
				backupStatus = `Ошибка импорта: ${err}`;
			} finally {
				importBusy = false;
			}
		};
		input.click();
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
		// Extended palette
		'#0ea5e9', '#d97706', '#dc2626', '#7c3aed',
		// Monochrome — selecting this also activates monochrome palette mode
		'#1a1a1a',
	];

	function selectColor(color: string) {
		seedColor.set(color);
		// Black/near-black swatch activates monochrome mode; any other colour resets to default
		paletteMode.set(color === '#1a1a1a' ? 'monochrome' : 'default');
	}

	const presets: { id: AppPreset; label: string }[] = [
		{ id: 'flowers',   label: $t('preset_flowers') },
		{ id: 'ochokochi', label: $t('preset_ochokochi') },
		{ id: 'balanced',  label: $t('preset_balanced') },
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

		<!-- UI Scale slider -->
		<div class="scale-row">
			<label class="scale-label" for="ui-scale-input">{$t('label_ui_scale')} — {$uiScale.toFixed(1)}×</label>
			<input
				id="ui-scale-input"
				type="range"
				min="0.8"
				max="1.4"
				step="0.05"
				value={$uiScale}
				oninput={(e) => uiScale.set(parseFloat((e.target as HTMLInputElement).value))}
				class="scale-slider"
			/>
			<div class="scale-markers">
				<span>0.8×</span><span>1.0×</span><span>1.2×</span><span>1.4×</span>
			</div>
		</div>

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

		<!-- Palette mode toggle -->
		<div class="palette-toggle-row">
			<span class="scale-label">{$t('label_palette_mode') ?? 'Palette'}</span>
			<div class="mode-toggle-row">
				<button
					class="mode-btn"
					class:mode-active={$paletteMode === 'default'}
					onclick={() => paletteMode.set('default')}
				>
					{$t('label_palette_default') ?? 'Default'}
				</button>
				<button
					class="mode-btn"
					class:mode-active={$paletteMode === 'monochrome'}
					onclick={() => paletteMode.set('monochrome')}
				>
					{$t('label_palette_monochrome') ?? 'Monochrome'}
				</button>
			</div>
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
					class:active={currentColor === color}
					class:swatch-mono={color === '#1a1a1a'}
					style:background={color}
					onclick={() => selectColor(color)}
					aria-label={color === '#1a1a1a' ? 'Монохромный' : `Цвет ${color}`}
					title={color === '#1a1a1a' ? 'Чёрно-белый' : color}
				></button>
			{/each}
		</div>
	</section>

	<!-- ── Currency ──────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_currency')}</h2>
		<p class="hint">{$t('hint_currency')}</p>
		<GlassDropdown
			items={CURRENCIES.map(c => ({ value: c.code, label: `${c.symbol} ${c.code} — ${c.name}` }))}
			bind:value={$globalCurrency}
			placeholder="— Currency —"
		/>
	</section>

	<!-- ── Language ───────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_language')}</h2>
		<GlassDropdown
			items={[{ value: 'ru', label: 'Русский' }, { value: 'en', label: 'English' }]}
			bind:value={$locale}
		/>
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

	<!-- ── Flower Constants (preset=flowers only) ────────────── -->
	{#if $preset === 'flowers'}
	<section class="section" id="flowers-constants">
		<h2>🌸 Параметры цветочного бизнеса</h2>
		<div class="constants-form">
			<div class="const-field">
				<label class="const-label" for="fpp">Цветков в упаковке (шт.)</label>
				<input id="fpp" class="const-input" type="number" step="1" min="1" bind:value={constantsDraft.flowers_per_pack} />
			</div>
			<div class="const-field">
				<label class="const-label" for="wpf">Вес стебля (кг)</label>
				<input id="wpf" class="const-input" type="number" step="0.001" min="0" bind:value={constantsDraft.weight_per_flower} />
			</div>
			<div class="const-field">
				<label class="const-label" for="ppp">Цена упаковки</label>
				<input id="ppp" class="const-input" type="number" step="1" min="0" bind:value={constantsDraft.price_per_pack} />
			</div>
			<div class="const-field">
				<label class="const-label" for="ppf">Цена стебля</label>
				<input id="ppf" class="const-input" type="number" step="0.5" min="0" bind:value={constantsDraft.price_per_flower} />
			</div>
		</div>
		<div class="const-field" style="margin-top:12px">
			<span class="const-label">Логика ценообразования</span>
			<div class="pricing-mode-row">
				{#each [['pack','По упаковкам'],['stem','По стеблям'],['mixed','Смешанная']] as [val, lbl]}
					<label class="pricing-mode-option">
						<input
							type="radio"
							name="pricing_mode"
							value={val}
							checked={constantsDraft.pricing_mode === val}
							onchange={() => (constantsDraft.pricing_mode = val as PricingMode)}
						/>
						{lbl}
					</label>
				{/each}
			</div>
		</div>
		<button class="btn-primary" style="margin-top:16px" onclick={handleSaveConstants} disabled={constantsSaving}>
			{constantsSaving ? '…' : $t('action_save')}
		</button>
	</section>
	{/if}

	<!-- ── Backup ─────────────────────────────────────────────── -->
	<section class="section">
		<h2>{$t('label_backup')}</h2>
		<p class="hint">{$t('hint_backup_description')}</p>
		<div class="backup-actions">
			<button class="btn-primary" onclick={handleExportBackup}>
				📥 {$t('action_export_backup')}
			</button>
			<button class="btn-secondary" onclick={handleImportBackup} disabled={importBusy}>
				{importBusy ? '…' : '📤 Восстановить из файла'}
			</button>
		</div>
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

	/* ── UI Scale slider ─────────── */
	.scale-row {
		margin-bottom: 20px;
	}

	.scale-label {
		display: block;
		font-size: 0.85rem;
		color: var(--color-on-surface);
		margin-bottom: 10px;
		font-weight: 500;
	}

	.scale-slider {
		width: 100%;
		accent-color: var(--color-primary);
		cursor: pointer;
		height: 4px;
		border-radius: 2px;
	}

	.scale-markers {
		display: flex;
		justify-content: space-between;
		margin-top: 6px;
		font-size: 0.7rem;
		color: var(--color-outline);
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

	.palette-toggle-row {
		margin-top: 16px;
		display: flex;
		flex-direction: column;
		gap: 8px;
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

	/* Monochrome swatch — visible border so the dark circle reads on dark bg */
	.swatch-mono {
		border-color: rgba(255, 255, 255, 0.25);
		background: linear-gradient(135deg, #2a2a2a 0%, #0a0a0a 100%) !important;
	}
	.swatch-mono.active { border-color: var(--color-on-surface); }

	/* ── Language ─────────────── */

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
	}

	.backup-actions {
		display: flex;
		gap: 10px;
		flex-wrap: wrap;
		align-items: center;
	}

	.btn-secondary {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		padding: 8px 20px;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.875rem;
		font-family: inherit;
		transition: background 0.15s;
	}

	.btn-secondary:hover { background: var(--glass-bg-hover); }
	.btn-secondary:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

	/* ── Flower constants form ─── */
	.constants-form {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
		margin-bottom: 4px;
	}

	.const-field {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.const-label {
		font-size: 0.7rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-weight: 500;
	}

	.const-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 8px 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.2s;
	}

	.const-input:focus { border-color: var(--color-primary); }

	.pricing-mode-row {
		display: flex;
		gap: 16px;
		flex-wrap: wrap;
		margin-top: 4px;
	}

	.pricing-mode-option {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.85rem;
		color: var(--color-on-surface);
		cursor: pointer;
	}

	.pricing-mode-option input[type="radio"] {
		accent-color: var(--color-primary);
		cursor: pointer;
	}
</style>
