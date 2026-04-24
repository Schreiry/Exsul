<script lang="ts">
	import { contacts } from '$lib/stores/contacts';
	import { orders } from '$lib/stores/orders';
	import { inventory } from '$lib/stores/inventory';
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { t } from '$lib/stores/i18n';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { commands } from '$lib/tauri/commands';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { appDataDir, join } from '@tauri-apps/api/path';
	import { printAllOrders } from '$lib/utils/print';
	import type {
		Contact,
		ContactLocation,
		Order,
		PackAssignment,
		PackagingLogEntry,
	} from '$lib/tauri/types';

	interface Props {
		contactId: string;
		onclose: () => void;
		/** Called when the user clicks an order row inside history. */
		onselectorder?: (order: Order) => void;
		/** Start in edit mode with the name field focused (used for quick-create). */
		autoEdit?: boolean;
	}

	let { contactId, onclose, onselectorder, autoEdit = false }: Props = $props();
	let nameInputEl = $state<HTMLInputElement | null>(null);

	let contact = $state<Contact | null>(null);
	let locations = $state<ContactLocation[]>([]);
	let history = $state<Order[]>([]);
	let photoSrc = $state<string>('');
	let loading = $state(true);

	let editMode = $state(false);
	let savingProfile = $state(false);

	type ProfileDraft = {
		name: string;
		surname: string;
		email: string;
		phone: string;
		company: string;
		notes: string;
		card_color: string;
	};
	let draft = $state<ProfileDraft>({
		name: '',
		surname: '',
		email: '',
		phone: '',
		company: '',
		notes: '',
		card_color: '',
	});

	// New-location form state (inline at the top of the Addresses section)
	let newLocLabel = $state('');
	let newLocAddress = $state('');
	let addingLocation = $state(false);

	async function loadAll() {
		loading = true;
		try {
			[contact, locations, history] = await Promise.all([
				commands.getContact(contactId),
				commands.getContactLocations(contactId),
				commands.getOrdersForContact(contactId),
			]);
			if (contact?.photo_path) {
				try {
					const base = await appDataDir();
					const full = await join(base, contact.photo_path);
					photoSrc = convertFileSrc(full);
				} catch (e) {
					console.warn('Failed to resolve photo path:', e);
					photoSrc = '';
				}
			} else {
				photoSrc = '';
			}
		} catch (e) {
			console.error('Failed to load contact detail:', e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void contactId;
		loadAll().then(() => {
			// Quick-create path: caller passed `autoEdit` to drop the user straight
			// into the edit form with the name field focused, saving them a click
			// and making it obvious what to fill in first.
			if (autoEdit && contact) {
				enterEdit();
				queueMicrotask(() => nameInputEl?.focus());
			}
		});
	});

	function enterEdit() {
		if (!contact) return;
		draft = {
			name: contact.name,
			surname: contact.surname ?? '',
			email: contact.email ?? '',
			phone: contact.phone ?? '',
			company: contact.company ?? '',
			notes: contact.notes ?? '',
			card_color: contact.card_color ?? '',
		};
		editMode = true;
	}

	async function saveEdit() {
		if (!contact || !draft.name.trim()) return;
		savingProfile = true;
		try {
			await contacts.update({
				contact_id: contact.id,
				name: draft.name.trim(),
				surname: draft.surname.trim() || undefined,
				email: draft.email.trim() || undefined,
				phone: draft.phone.trim() || undefined,
				company: draft.company.trim() || undefined,
				notes: draft.notes.trim() || undefined,
				card_color: draft.card_color.trim() || undefined,
				clear_card_color: !draft.card_color.trim(),
			});
			await loadAll();
			editMode = false;
		} catch (e) {
			console.error('Failed to save contact:', e);
			alert(String(e));
		} finally {
			savingProfile = false;
		}
	}

	async function addLocation() {
		if (!contact || !newLocAddress.trim()) return;
		addingLocation = true;
		try {
			await contacts.addLocation({
				contact_id: contact.id,
				label: newLocLabel.trim() || undefined,
				address: newLocAddress.trim(),
			});
			newLocLabel = '';
			newLocAddress = '';
			locations = await commands.getContactLocations(contact.id);
		} catch (e) {
			console.error('Failed to add location:', e);
			alert(String(e));
		} finally {
			addingLocation = false;
		}
	}

	async function setDefault(loc: ContactLocation) {
		if (loc.is_default) return;
		try {
			await contacts.setDefaultLocation(loc.id);
			locations = await commands.getContactLocations(contact!.id);
		} catch (e) {
			console.error('Failed to set default:', e);
		}
	}

	async function removeLocation(loc: ContactLocation) {
		if (!confirm($t('action_delete') + '?')) return;
		try {
			await contacts.removeLocation(loc.id);
			locations = await commands.getContactLocations(contact!.id);
		} catch (e) {
			console.error('Failed to delete location:', e);
		}
	}

	async function uploadPhoto() {
		if (!contact) return;
		try {
			const picked = await openDialog({
				multiple: false,
				filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
			});
			if (!picked || typeof picked !== 'string') return;
			await contacts.uploadPhoto(contact.id, picked);
			await loadAll();
		} catch (e) {
			console.error('Failed to upload photo:', e);
			alert(String(e));
		}
	}

	async function handleDelete() {
		if (!contact) return;
		if (!confirm($t('contact_confirm_delete', { name: contact.name }))) return;
		try {
			await contacts.remove(contact.id);
			onclose();
		} catch (e) {
			console.error('Failed to delete contact:', e);
		}
	}

	// Lazy-load reference data the first time the user opens print. The flowers
	// page usually has these loaded already, but reaching this modal from
	// elsewhere (e.g. a future contacts-only page) shouldn't print blanks.
	let printing = $state(false);
	async function handlePrintAll() {
		if (!contact || history.length === 0 || printing) return;
		printing = true;
		try {
			if ($flowerSorts.length === 0) await flowerSorts.load();
			if ($inventory.length === 0) await inventory.load();
			if (!$flowerConstants || $flowerConstants.flowers_per_pack === 0) {
				await flowerConstants.load();
			}

			// Pre-fetch pack_assignments and packaging_log per order so the
			// rendered sheets carry the full warehouse context — without these,
			// legacy orders with empty order_items print blank rows. Matches the
			// exact plumbing /orders uses for the registry button.
			const packAssignmentsByOrder: Record<string, PackAssignment[]> = {};
			const packagingLogByOrder: Record<string, PackagingLogEntry[]> = {};
			await Promise.all(
				history.map(async (o) => {
					const [pa, pl] = await Promise.all([
						commands.getPackAssignments(o.id).catch(() => []),
						commands.getPackagingLogByOrder(o.id).catch(() => []),
					]);
					packAssignmentsByOrder[o.id] = pa;
					packagingLogByOrder[o.id] = pl;
				})
			);

			await printAllOrders(
				history,
				(id) => orders.getItems(id),
				$flowerSorts,
				$inventory,
				$flowerConstants,
				$globalCurrency,
				$t,
				{ packAssignmentsByOrder, packagingLogByOrder }
			);
		} catch (e) {
			console.error('Failed to print contact orders:', e);
			alert(String(e));
		} finally {
			printing = false;
		}
	}

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdrop} role="dialog" aria-modal="true">
	<div class="modal-panel">
		<button class="btn-close" type="button" onclick={onclose} aria-label="Закрыть">✕</button>

		{#if loading}
			<div class="loading">…</div>
		{:else if !contact}
			<div class="loading">—</div>
		{:else}
			<div class="header">
				<button class="avatar-btn" type="button" onclick={uploadPhoto} title={$t('contact_upload_photo')}>
					{#if photoSrc}
						<img src={photoSrc} alt="" />
					{:else}
						<span class="initials">
							{(contact.name || '?').trim().charAt(0).toUpperCase()}
						</span>
					{/if}
				</button>
				<div class="header-info">
					{#if editMode}
						<input
							bind:this={nameInputEl}
							class="edit-input edit-name"
							type="text"
							bind:value={draft.name}
							placeholder={$t('label_customer_name')}
						/>
					{:else}
						<h2 class="title">{contact.name}{contact.surname ? ` ${contact.surname}` : ''}</h2>
						{#if contact.company}
							<p class="subtitle">{contact.company}</p>
						{/if}
					{/if}
					<div class="metrics">
						<span>{$t('contact_orders_count', { n: contact.order_count })}</span>
						{#if contact.total_spent > 0}
							<span class="metric-total">
								· {$t('contact_total_spent')}:
								<strong>{formatAmount(contact.total_spent, $globalCurrency)}</strong>
							</span>
						{/if}
					</div>
				</div>
				<div class="header-actions">
					{#if editMode}
						<button class="btn-ghost" onclick={() => (editMode = false)}>{$t('action_cancel')}</button>
						<button class="btn-primary" disabled={savingProfile || !draft.name.trim()} onclick={saveEdit}>
							{savingProfile ? '…' : $t('action_save')}
						</button>
					{:else}
						<button
							class="btn-ghost btn-print"
							onclick={handlePrintAll}
							disabled={printing || history.length === 0}
							title={$t('contact_print_all_orders')}
							aria-label={$t('contact_print_all_orders')}
						>
							<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="6 9 6 2 18 2 18 9"/>
								<path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>
								<rect x="6" y="14" width="12" height="8"/>
							</svg>
							{printing ? '…' : $t('contact_print_all_orders')}
						</button>
						<button class="btn-ghost" onclick={enterEdit}>{$t('label_edit_item')}</button>
						<button class="btn-ghost btn-danger" onclick={handleDelete}>{$t('action_delete')}</button>
					{/if}
				</div>
			</div>

			<div class="body">
				<!-- Profile -->
				<section class="section">
					<h3>{$t('contact_section_profile')}</h3>
					{#if editMode}
						<div class="profile-grid">
							<label class="field">
								<span>{$t('contact_label_surname')}</span>
								<input class="edit-input" type="text" bind:value={draft.surname} />
							</label>
							<label class="field">
								<span>{$t('label_customer_phone')}</span>
								<input class="edit-input" type="tel" bind:value={draft.phone} />
							</label>
							<label class="field">
								<span>{$t('label_customer_email')}</span>
								<input class="edit-input" type="email" bind:value={draft.email} />
							</label>
							<label class="field">
								<span>{$t('contact_label_company')}</span>
								<input class="edit-input" type="text" bind:value={draft.company} />
							</label>
							<label class="field full">
								<span>{$t('label_notes')}</span>
								<textarea class="edit-input" rows="2" bind:value={draft.notes}></textarea>
							</label>
							<label class="field">
								<span>{$t('label_card_color')}</span>
								<div class="color-row">
									<input
										class="color-swatch"
										type="color"
										value={draft.card_color || '#888888'}
										oninput={(e) => (draft.card_color = (e.currentTarget as HTMLInputElement).value)}
									/>
									<input class="edit-input" type="text" bind:value={draft.card_color} placeholder="#rrggbb" />
									{#if draft.card_color}
										<button
											type="button"
											class="btn-ghost"
											onclick={() => (draft.card_color = '')}
											title={$t('action_reset_to_auto')}
										>↺</button>
									{/if}
								</div>
							</label>
						</div>
					{:else}
						<div class="profile-display">
							{#if contact.phone}<div><span class="lbl">{$t('label_customer_phone')}:</span> {contact.phone}</div>{/if}
							{#if contact.email}<div><span class="lbl">{$t('label_customer_email')}:</span> {contact.email}</div>{/if}
							{#if contact.company}<div><span class="lbl">{$t('contact_label_company')}:</span> {contact.company}</div>{/if}
							{#if contact.notes}<div class="notes">{contact.notes}</div>{/if}
						</div>
					{/if}
				</section>

				<!-- Locations -->
				<section class="section">
					<h3>{$t('contact_section_locations')}</h3>

					<div class="add-loc-row">
						<input
							class="edit-input tiny"
							type="text"
							placeholder={$t('contact_label_label')}
							bind:value={newLocLabel}
						/>
						<input
							class="edit-input"
							type="text"
							placeholder={$t('contact_label_address')}
							bind:value={newLocAddress}
						/>
						<button
							class="btn-primary"
							onclick={addLocation}
							disabled={addingLocation || !newLocAddress.trim()}
						>
							{addingLocation ? '…' : '+'}
						</button>
					</div>

					{#if locations.length === 0}
						<p class="empty">—</p>
					{:else}
						<ul class="locations">
							{#each locations as loc (loc.id)}
								<li class="loc-row" class:default={loc.is_default}>
									<div class="loc-main">
										{#if loc.label}<span class="loc-label">{loc.label}</span>{/if}
										<span class="loc-addr">{loc.address}</span>
										{#if loc.is_default}
											<span class="badge-default">{$t('contact_is_default')}</span>
										{/if}
									</div>
									<div class="loc-actions">
										{#if !loc.is_default}
											<button class="btn-ghost" onclick={() => setDefault(loc)}>
												{$t('contact_set_default')}
											</button>
										{/if}
										<button class="btn-ghost btn-danger" onclick={() => removeLocation(loc)}>
											✕
										</button>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</section>

				<!-- History -->
				<section class="section">
					<h3>{$t('contact_section_orders')}</h3>
					{#if history.length === 0}
						<p class="empty">{$t('contact_no_orders')}</p>
					{:else}
						<ul class="history">
							{#each history as o (o.id)}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<li class="hist-row" role="button" tabindex="0"
									onclick={() => onselectorder?.(o)}
									onkeydown={(e) => e.key === 'Enter' && onselectorder?.(o)}
								>
									<div class="hist-left">
										<span class="hist-date">
											{new Date(o.created_at).toLocaleDateString('ru', {
												day: '2-digit', month: 'short', year: 'numeric',
											})}
										</span>
										<span class="hist-status">{$t('status_' + o.status)}</span>
									</div>
									<span class="hist-amount">{formatAmount(o.total_amount, $globalCurrency)}</span>
								</li>
							{/each}
						</ul>
					{/if}
				</section>
			</div>
		{/if}
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0,0,0,0.55);
		backdrop-filter: blur(8px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		padding: 16px;
	}

	.modal-panel {
		width: 100%;
		max-width: 720px;
		max-height: 90vh;
		background: var(--color-surface, #18181b);
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 24px;
		overflow-y: auto;
		padding: 24px;
		box-shadow: 0 32px 80px rgba(0,0,0,0.55);
		position: relative;
	}

	.btn-close {
		position: absolute;
		top: 12px; right: 12px;
		background: none; border: none;
		color: var(--color-outline);
		font-size: 1.1rem;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 8px;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	.loading { padding: 60px 0; text-align: center; color: var(--color-outline); }

	.header {
		display: flex;
		gap: 16px;
		align-items: center;
		margin-bottom: 20px;
	}
	.avatar-btn {
		width: 72px; height: 72px;
		border-radius: 50%;
		background: color-mix(in srgb, var(--color-primary) 14%, transparent);
		border: 1px solid var(--glass-border);
		cursor: pointer;
		overflow: hidden;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
	}
	.avatar-btn img { width: 100%; height: 100%; object-fit: cover; }
	.avatar-btn .initials {
		font-size: 1.6rem;
		font-weight: 700;
		color: var(--color-primary);
	}
	.avatar-btn:hover { opacity: 0.85; }

	.header-info { flex: 1; min-width: 0; }
	.title { font-size: 1.15rem; margin: 0; color: var(--color-on-surface); }
	.subtitle { font-size: 0.8rem; color: var(--color-outline); margin: 2px 0 0; }
	.metrics { font-size: 0.78rem; color: var(--color-outline); margin-top: 4px; }
	.metrics strong { color: var(--color-primary); font-weight: 700; }
	.metric-total { margin-left: 4px; }

	.header-actions { display: flex; gap: 6px; flex-shrink: 0; }

	.section { margin-top: 22px; }
	.section h3 {
		font-size: 0.78rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-outline);
		margin: 0 0 10px;
	}

	.edit-input, .color-swatch {
		background: var(--color-surface-container-high, var(--glass-bg));
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 8px;
		padding: 8px 10px;
		font-size: 0.85rem;
		color: var(--color-on-surface);
		outline: none;
		font-family: inherit;
		width: 100%;
		box-sizing: border-box;
	}
	.edit-input:focus { border-color: var(--color-primary); }
	.edit-input.tiny { max-width: 120px; }
	.edit-name { font-size: 1rem; font-weight: 600; }

	.profile-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}
	.field { display: flex; flex-direction: column; gap: 4px; }
	.field span { font-size: 0.7rem; text-transform: uppercase; color: var(--color-outline); }
	.field.full { grid-column: 1 / -1; }

	.color-row { display: flex; gap: 6px; align-items: center; }
	.color-swatch {
		width: 40px; height: 34px;
		padding: 0;
		flex-shrink: 0;
	}

	.profile-display {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 0.85rem;
		color: var(--color-on-surface);
	}
	.lbl { color: var(--color-outline); margin-right: 4px; }
	.notes { opacity: 0.8; white-space: pre-wrap; margin-top: 4px; }

	.add-loc-row {
		display: flex;
		gap: 6px;
		align-items: center;
		margin-bottom: 8px;
	}
	.add-loc-row .edit-input { flex: 1; }
	.add-loc-row .btn-primary { flex-shrink: 0; padding: 8px 14px; }

	.locations { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
	.loc-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		background: var(--glass-bg, rgba(255,255,255,0.04));
		border: 1px solid var(--glass-border);
		border-radius: 8px;
	}
	.loc-row.default { border-color: color-mix(in srgb, var(--color-primary) 40%, transparent); }
	.loc-main { flex: 1; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
	.loc-label {
		font-size: 0.72rem;
		background: var(--glass-bg);
		padding: 2px 8px;
		border-radius: 20px;
		color: var(--color-outline);
	}
	.loc-addr { font-size: 0.85rem; color: var(--color-on-surface); }
	.badge-default {
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-primary);
		background: color-mix(in srgb, var(--color-primary) 12%, transparent);
		padding: 2px 8px;
		border-radius: 20px;
	}
	.loc-actions { display: flex; gap: 4px; flex-shrink: 0; }

	.history { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
	.hist-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 10px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		cursor: pointer;
		transition: background 0.1s;
	}
	.hist-row:hover { background: var(--glass-bg-hover, rgba(255,255,255,0.08)); }
	.hist-left { display: flex; gap: 8px; align-items: baseline; }
	.hist-date { font-weight: 600; color: var(--color-on-surface); font-size: 0.85rem; }
	.hist-status { font-size: 0.72rem; color: var(--color-outline); }
	.hist-amount { font-weight: 700; color: var(--color-primary); font-size: 0.88rem; }

	.empty { color: var(--color-outline); font-size: 0.85rem; opacity: 0.7; margin: 0; }

	.btn-ghost {
		background: none;
		border: none;
		color: var(--color-on-surface);
		opacity: 0.6;
		cursor: pointer;
		padding: 6px 10px;
		border-radius: 6px;
		font-size: 0.82rem;
	}
	.btn-ghost:hover { opacity: 1; background: var(--glass-bg-hover); }
	.btn-ghost.btn-danger:hover { color: var(--color-alert-red); }
	.btn-ghost.btn-print {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.btn-ghost:disabled { opacity: 0.3; cursor: not-allowed; }

	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary, white);
		border: none;
		border-radius: 8px;
		padding: 8px 14px;
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
	}
	.btn-primary:hover { opacity: 0.85; }
	.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

	@media (max-width: 540px) {
		.profile-grid { grid-template-columns: 1fr; }
	}
</style>
