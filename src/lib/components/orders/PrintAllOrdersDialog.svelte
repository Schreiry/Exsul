<script lang="ts">
	import { t } from '$lib/stores/i18n';

	interface Props {
		earliestDate: string | null;
		onconfirm: (range: { dateFrom: string; dateTo: string }) => void;
		onclose: () => void;
	}

	let { earliestDate, onconfirm, onclose }: Props = $props();

	function toYmd(iso: string): string {
		const d = new Date(iso);
		if (Number.isNaN(d.getTime())) return '';
		const y = d.getFullYear();
		const m = String(d.getMonth() + 1).padStart(2, '0');
		const day = String(d.getDate()).padStart(2, '0');
		return `${y}-${m}-${day}`;
	}

	function minusDays(days: number): string {
		const d = new Date();
		d.setDate(d.getDate() - days);
		return toYmd(d.toISOString());
	}

	const todayYmd = toYmd(new Date().toISOString());
	const earliestYmd = earliestDate ? toYmd(earliestDate) : todayYmd;

	let dateFrom = $state(earliestYmd);
	let dateTo = $state(todayYmd);

	const invalid = $derived(
		!dateFrom || !dateTo || new Date(dateFrom).getTime() > new Date(dateTo).getTime()
	);

	function applyPreset(days: number) {
		dateFrom = minusDays(days);
		dateTo = todayYmd;
	}

	function applyAllTime() {
		dateFrom = earliestYmd;
		dateTo = todayYmd;
	}

	function handleConfirm() {
		if (invalid) return;
		onconfirm({ dateFrom, dateTo });
	}

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
		if (e.key === 'Enter' && !invalid) handleConfirm();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdrop} role="dialog" aria-modal="true">
	<div class="modal-panel">
		<div class="modal-header">
			<h2>{$t('print_dialog_title')}</h2>
			<button class="btn-close" type="button" onclick={onclose} aria-label={$t('action_close') || 'Close'}>&#10005;</button>
		</div>

		<div class="modal-body">
			<div class="hint">
				{#if earliestDate}
					{$t('print_dialog_hint_earliest').replace('{date}', earliestYmd)}
				{:else}
					{$t('print_dialog_hint_no_orders')}
				{/if}
			</div>

			<div class="presets">
				<button type="button" class="preset-btn" onclick={() => applyPreset(7)}>
					{$t('print_dialog_preset_7')}
				</button>
				<button type="button" class="preset-btn" onclick={() => applyPreset(14)}>
					{$t('print_dialog_preset_14')}
				</button>
				<button type="button" class="preset-btn" onclick={() => applyPreset(30)}>
					{$t('print_dialog_preset_30')}
				</button>
				<button type="button" class="preset-btn preset-all" onclick={applyAllTime}>
					{$t('print_dialog_preset_all')}
				</button>
			</div>

			<div class="date-row">
				<label class="date-field">
					<span class="date-label">{$t('print_dialog_date_from')}</span>
					<input
						type="date"
						bind:value={dateFrom}
						min={earliestYmd}
						max={todayYmd}
						class="date-input"
					/>
				</label>
				<label class="date-field">
					<span class="date-label">{$t('print_dialog_date_to')}</span>
					<input
						type="date"
						bind:value={dateTo}
						min={dateFrom || earliestYmd}
						max={todayYmd}
						class="date-input"
					/>
				</label>
			</div>

			{#if invalid}
				<div class="warn"><span>&#9888;</span> {$t('print_dialog_invalid_range')}</div>
			{/if}
		</div>

		<div class="modal-footer">
			<button type="button" class="btn-cancel" onclick={onclose}>
				{$t('action_cancel')}
			</button>
			<button type="button" class="btn-confirm" onclick={handleConfirm} disabled={invalid}>
				<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<polyline points="6 9 6 2 18 2 18 9"/>
					<path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>
					<rect x="6" y="14" width="12" height="8"/>
				</svg>
				{$t('print_dialog_confirm')}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(10px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2200;
		padding: 16px;
	}

	.modal-panel {
		background: var(--color-surface);
		border: 1px solid var(--glass-border);
		border-radius: 20px;
		width: 100%;
		max-width: 520px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		box-shadow: 0 32px 80px rgba(0, 0, 0, 0.5);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 18px 22px;
		border-bottom: 1px solid var(--glass-border);
		flex-shrink: 0;
	}

	.modal-header h2 {
		font-size: 1.05rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0;
	}

	.btn-close {
		background: none;
		border: none;
		color: var(--color-outline);
		font-size: 1rem;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 8px;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	.modal-body {
		padding: 18px 22px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.hint {
		font-size: 0.8rem;
		color: var(--color-outline);
		line-height: 1.4;
	}

	.presets {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.preset-btn {
		flex: 1 1 auto;
		min-width: 90px;
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		color: var(--color-on-surface);
		border-radius: 10px;
		padding: 8px 12px;
		font-size: 0.825rem;
		font-weight: 500;
		cursor: pointer;
		font-family: inherit;
		transition: background 0.15s, border-color 0.15s;
	}
	.preset-btn:hover {
		background: var(--color-surface-container);
		border-color: var(--color-primary);
	}
	.preset-btn.preset-all {
		border-style: dashed;
	}

	.date-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}

	.date-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.date-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.date-input {
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 10px;
		padding: 9px 12px;
		color: var(--color-on-surface);
		font-size: 0.9rem;
		font-family: inherit;
		outline: none;
		color-scheme: dark;
	}
	.date-input:focus {
		border-color: var(--color-primary);
	}

	:global([data-theme="light"]) .date-input {
		color-scheme: light;
	}

	.warn {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--color-alert-red, #ef4444);
		background: rgba(239, 68, 68, 0.08);
		border: 1px solid var(--color-alert-red, #ef4444);
		border-radius: 8px;
		padding: 6px 10px;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
		padding: 14px 22px 18px;
		border-top: 1px solid var(--glass-border);
	}

	.btn-cancel {
		background: none;
		border: 1px solid var(--color-outline-variant);
		color: var(--color-on-surface);
		border-radius: 10px;
		padding: 9px 18px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		font-family: inherit;
	}
	.btn-cancel:hover { background: var(--color-surface-container); }

	.btn-confirm {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		border: none;
		border-radius: 10px;
		padding: 9px 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		font-family: inherit;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		transition: opacity 0.15s;
	}
	.btn-confirm:hover { opacity: 0.88; }
	.btn-confirm:disabled { opacity: 0.4; cursor: not-allowed; }

	:global([data-theme="light"]) .modal-panel {
		background: var(--color-surface, #fafafa);
	}
</style>
