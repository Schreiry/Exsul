<script lang="ts">
	import { contacts } from '$lib/stores/contacts';
	import { t } from '$lib/stores/i18n';
	import type { Contact } from '$lib/tauri/types';

	interface Props {
		/** Currently selected contact id (bound). Empty string = none. */
		selectedId?: string;
		/** Fires when the user picks an existing contact or quick-creates one. */
		onselect?: (contact: Contact) => void;
		/** Fires when the user clears the selection. */
		onclear?: () => void;
		/** Allow the user to type a free-form name that isn't in the directory. */
		allowFreeInput?: boolean;
		/** Mirror of the free-form name typed while no contact is selected. */
		freeName?: string;
		onfreeName?: (value: string) => void;
	}

	let {
		selectedId = '',
		onselect,
		onclear,
		allowFreeInput = true,
		freeName = '',
		onfreeName,
	}: Props = $props();

	// Load the contact list once per component mount. The store keeps state,
	// so re-entering the page doesn't re-fetch unnecessarily.
	$effect(() => {
		if ($contacts.length === 0) contacts.load();
	});

	// Resolve the Contact matching selectedId (if any) so we can render the
	// chip inline even if the parent only knows the id.
	const selectedContact = $derived(
		selectedId ? $contacts.find((c) => c.id === selectedId) ?? null : null
	);

	let query = $state('');
	let dropdownOpen = $state(false);
	let inputEl = $state<HTMLInputElement | null>(null);

	// Quick-create panel state — appears when search yields no match
	let quickCreateMode = $state(false);
	let quickName = $state('');
	let quickPhone = $state('');
	let quickAddress = $state('');
	let saving = $state(false);

	const normalized = $derived(query.trim().toLowerCase());
	const filtered = $derived.by(() => {
		if (!normalized) return $contacts.slice(0, 20);
		return $contacts.filter(
			(c) =>
				c.name.toLowerCase().includes(normalized) ||
				(c.surname ?? '').toLowerCase().includes(normalized) ||
				(c.phone ?? '').toLowerCase().includes(normalized) ||
				(c.email ?? '').toLowerCase().includes(normalized) ||
				(c.company ?? '').toLowerCase().includes(normalized)
		);
	});

	function openDropdown() {
		dropdownOpen = true;
	}

	function closeDropdown() {
		dropdownOpen = false;
		quickCreateMode = false;
	}

	function selectContact(c: Contact) {
		onselect?.(c);
		query = '';
		closeDropdown();
	}

	function clearSelection() {
		onclear?.();
		query = '';
		inputEl?.focus();
	}

	function beginQuickCreate() {
		quickCreateMode = true;
		quickName = query.trim();
	}

	async function submitQuickCreate() {
		if (!quickName.trim()) return;
		saving = true;
		try {
			const id = await contacts.create({
				name: quickName.trim(),
				phone: quickPhone.trim() || undefined,
				default_address: quickAddress.trim() || undefined,
			});
			// Reload pulled a fresh list — find the new contact and select it.
			const created = $contacts.find((c) => c.id === id);
			if (created) selectContact(created);
			quickName = '';
			quickPhone = '';
			quickAddress = '';
		} catch (e) {
			console.error('Quick-create contact failed:', e);
		} finally {
			saving = false;
		}
	}

	function handleBlur(e: FocusEvent) {
		// Delay close so click handlers on dropdown items fire first
		const next = (e.relatedTarget as HTMLElement | null);
		if (next && next.closest('.picker-root')) return;
		setTimeout(closeDropdown, 120);
	}
</script>

<div class="picker-root">
	{#if selectedContact}
		<div class="selected-chip">
			<span class="chip-name">{selectedContact.name}</span>
			{#if selectedContact.phone}
				<span class="chip-phone">{selectedContact.phone}</span>
			{/if}
			<button type="button" class="chip-clear" onclick={clearSelection} aria-label="x">✕</button>
		</div>
	{:else}
		<input
			bind:this={inputEl}
			class="picker-input"
			type="text"
			placeholder={$t('contact_picker_placeholder')}
			value={allowFreeInput ? (query || freeName) : query}
			oninput={(e) => {
				const v = (e.currentTarget as HTMLInputElement).value;
				query = v;
				if (allowFreeInput) onfreeName?.(v);
				openDropdown();
			}}
			onfocus={openDropdown}
			onblur={handleBlur}
		/>
	{/if}

	{#if dropdownOpen && !selectedContact}
		<div class="picker-dropdown" role="listbox">
			{#if quickCreateMode}
				<div class="quick-create">
					<div class="quick-title">{$t('contacts_add')}</div>
					<input
						class="quick-input"
						type="text"
						placeholder={$t('label_customer_name')}
						bind:value={quickName}
					/>
					<input
						class="quick-input"
						type="tel"
						placeholder={$t('label_customer_phone')}
						bind:value={quickPhone}
					/>
					<input
						class="quick-input"
						type="text"
						placeholder={$t('contact_label_address')}
						bind:value={quickAddress}
					/>
					<div class="quick-actions">
						<button
							type="button"
							class="btn-quick-cancel"
							onclick={() => (quickCreateMode = false)}
						>
							{$t('action_cancel')}
						</button>
						<button
							type="button"
							class="btn-quick-save"
							onclick={submitQuickCreate}
							disabled={saving || !quickName.trim()}
						>
							{saving ? '…' : $t('action_save')}
						</button>
					</div>
				</div>
			{:else}
				{#if filtered.length === 0}
					<div class="dropdown-empty">
						{#if query.trim()}
							<button type="button" class="create-row" onclick={beginQuickCreate}>
								{$t('contact_picker_create_new', { query: query.trim() })}
							</button>
						{:else}
							<span class="empty-text">{$t('contacts_empty')}</span>
						{/if}
					</div>
				{:else}
					{#each filtered as c (c.id)}
						<button
							type="button"
							class="dropdown-row"
							onclick={() => selectContact(c)}
						>
							<div class="row-main">
								<span class="row-name">{c.name}</span>
								{#if c.surname}
									<span class="row-surname">{c.surname}</span>
								{/if}
							</div>
							<div class="row-meta">
								{#if c.phone}<span>{c.phone}</span>{/if}
								{#if c.order_count > 0}
									<span class="row-count">· {c.order_count}</span>
								{/if}
							</div>
						</button>
					{/each}
					{#if query.trim() && !filtered.some((c) => c.name.toLowerCase() === normalized)}
						<button type="button" class="create-row" onclick={beginQuickCreate}>
							{$t('contact_picker_create_new', { query: query.trim() })}
						</button>
					{/if}
				{/if}
			{/if}
		</div>
	{/if}
</div>

<style>
	.picker-root {
		position: relative;
		width: 100%;
	}

	.picker-input {
		width: 100%;
		background: var(--color-surface-container-high, var(--glass-bg));
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 8px;
		padding: 10px 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		outline: none;
		font-family: inherit;
		transition: border-color 0.15s;
		box-sizing: border-box;
	}
	.picker-input:focus { border-color: var(--color-primary); }

	.selected-chip {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
		border: 1px solid color-mix(in srgb, var(--color-primary) 30%, transparent);
		border-radius: 8px;
	}
	.chip-name {
		font-weight: 600;
		color: var(--color-on-surface);
		font-size: 0.88rem;
	}
	.chip-phone {
		font-size: 0.78rem;
		color: var(--color-outline);
	}
	.chip-clear {
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		padding: 2px 6px;
		font-size: 0.85rem;
		line-height: 1;
		margin-left: auto;
		border-radius: 6px;
	}
	.chip-clear:hover { color: var(--color-alert-red); background: rgba(0,0,0,0.1); }

	.picker-dropdown {
		position: absolute;
		top: calc(100% + 4px);
		left: 0; right: 0;
		max-height: 280px;
		overflow-y: auto;
		background: var(--color-surface-container, #1b1b1f);
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 10px;
		box-shadow: 0 12px 32px rgba(0,0,0,0.35);
		z-index: 20;
		padding: 4px;
	}

	.dropdown-row {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		background: none;
		border: none;
		padding: 8px 10px;
		border-radius: 8px;
		text-align: left;
		cursor: pointer;
		color: var(--color-on-surface);
	}
	.dropdown-row:hover { background: var(--glass-bg-hover, rgba(255,255,255,0.04)); }
	.row-main { display: flex; align-items: baseline; gap: 6px; }
	.row-name { font-weight: 600; font-size: 0.88rem; }
	.row-surname { font-size: 0.76rem; color: var(--color-outline); }
	.row-meta { display: flex; gap: 8px; font-size: 0.72rem; color: var(--color-outline); }
	.row-count { color: var(--color-primary); font-weight: 500; }

	.create-row {
		width: 100%;
		background: none;
		border: none;
		padding: 10px;
		margin-top: 2px;
		border-top: 1px dashed var(--glass-border);
		color: var(--color-primary);
		font-weight: 600;
		font-size: 0.85rem;
		text-align: left;
		cursor: pointer;
		border-radius: 0 0 8px 8px;
	}
	.create-row:hover {
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
	}

	.dropdown-empty {
		padding: 12px;
	}
	.empty-text { color: var(--color-outline); font-size: 0.85rem; }

	.quick-create {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
	}
	.quick-title {
		font-weight: 700;
		font-size: 0.85rem;
		color: var(--color-on-surface);
		margin-bottom: 4px;
	}
	.quick-input {
		background: var(--glass-bg, rgba(255,255,255,0.05));
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 6px;
		padding: 7px 10px;
		font-size: 0.82rem;
		color: var(--color-on-surface);
		outline: none;
	}
	.quick-input:focus { border-color: var(--color-primary); }
	.quick-actions { display: flex; gap: 6px; justify-content: flex-end; margin-top: 4px; }
	.btn-quick-cancel,
	.btn-quick-save {
		border: none;
		border-radius: 6px;
		padding: 6px 12px;
		font-size: 0.82rem;
		font-weight: 600;
		cursor: pointer;
	}
	.btn-quick-cancel { background: transparent; color: var(--color-outline); }
	.btn-quick-save {
		background: var(--color-primary);
		color: var(--color-on-primary, white);
	}
	.btn-quick-save:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
