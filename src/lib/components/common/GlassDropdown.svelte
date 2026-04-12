<script lang="ts">
	export interface DropdownItem {
		value: string;
		label: string;
	}

	let {
		items = [],
		value = $bindable(''),
		placeholder = '— Select —',
		disabled = false,
	}: {
		items: DropdownItem[];
		value: string;
		placeholder?: string;
		disabled?: boolean;
	} = $props();

	let open = $state(false);
	let highlightIndex = $state(-1);
	let triggerEl = $state<HTMLButtonElement | null>(null);
	let listEl = $state<HTMLUListElement | null>(null);
	let wrapperEl = $state<HTMLDivElement | null>(null);
	let flipUp = $state(false);

	const selectedLabel = $derived(items.find((i) => i.value === value)?.label ?? '');

	function toggle() {
		if (disabled) return;
		if (open) {
			close();
		} else {
			openDropdown();
		}
	}

	function openDropdown() {
		// Determine flip direction
		if (triggerEl) {
			const rect = triggerEl.getBoundingClientRect();
			const spaceBelow = window.innerHeight - rect.bottom;
			flipUp = spaceBelow < 240 && rect.top > spaceBelow;
		}
		open = true;
		highlightIndex = items.findIndex((i) => i.value === value);
		if (highlightIndex < 0) highlightIndex = 0;
		// Focus list after render
		requestAnimationFrame(() => listEl?.focus());
	}

	function close() {
		open = false;
		highlightIndex = -1;
		triggerEl?.focus();
	}

	function select(item: DropdownItem) {
		value = item.value;
		close();
	}

	function handleTriggerKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			if (!open) openDropdown();
		}
	}

	function handleListKeydown(e: KeyboardEvent) {
		switch (e.key) {
			case 'ArrowDown':
				e.preventDefault();
				highlightIndex = (highlightIndex + 1) % items.length;
				scrollToHighlighted();
				break;
			case 'ArrowUp':
				e.preventDefault();
				highlightIndex = (highlightIndex - 1 + items.length) % items.length;
				scrollToHighlighted();
				break;
			case 'Enter':
			case ' ':
				e.preventDefault();
				if (highlightIndex >= 0 && highlightIndex < items.length) {
					select(items[highlightIndex]);
				}
				break;
			case 'Escape':
				e.preventDefault();
				close();
				break;
			case 'Tab':
				close();
				break;
		}
	}

	function scrollToHighlighted() {
		requestAnimationFrame(() => {
			const el = listEl?.children[highlightIndex] as HTMLElement | undefined;
			el?.scrollIntoView({ block: 'nearest' });
		});
	}

	function handleClickOutside(e: MouseEvent) {
		if (open && wrapperEl && !wrapperEl.contains(e.target as Node)) {
			close();
		}
	}
</script>

<svelte:window onclick={handleClickOutside} />

<div class="glass-dropdown" bind:this={wrapperEl} class:disabled>
	<button
		bind:this={triggerEl}
		type="button"
		class="gd-trigger"
		class:open
		{disabled}
		aria-haspopup="listbox"
		aria-expanded={open}
		onclick={toggle}
		onkeydown={handleTriggerKeydown}
	>
		<span class="gd-trigger-text" class:placeholder={!selectedLabel}>
			{selectedLabel || placeholder}
		</span>
		<span class="gd-chevron" class:open>
			<svg width="12" height="12" viewBox="0 0 12 12" fill="none">
				<path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
		</span>
	</button>

	{#if open}
		<ul
			bind:this={listEl}
			class="gd-list"
			class:flip-up={flipUp}
			role="listbox"
			tabindex="-1"
			aria-activedescendant={highlightIndex >= 0 ? `gd-opt-${highlightIndex}` : undefined}
			onkeydown={handleListKeydown}
		>
			{#each items as item, i (item.value)}
				<li
					id="gd-opt-{i}"
					class="gd-option"
					class:highlighted={i === highlightIndex}
					class:selected={item.value === value}
					role="option"
					aria-selected={item.value === value}
					onclick={() => select(item)}
					onmouseenter={() => (highlightIndex = i)}
				>
					{item.label}
					{#if item.value === value}
						<span class="gd-check">
							<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
								<path d="M3 7L6 10L11 4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
							</svg>
						</span>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.glass-dropdown {
		position: relative;
		display: inline-flex;
		min-width: 160px;
	}

	.glass-dropdown.disabled {
		opacity: 0.4;
		pointer-events: none;
	}

	/* ── Trigger ── */
	.gd-trigger {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		padding: 8px 14px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		cursor: pointer;
		outline: none;
		transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
		backdrop-filter: blur(12px) saturate(140%);
		-webkit-backdrop-filter: blur(12px) saturate(140%);
	}

	.gd-trigger:hover {
		background: var(--glass-bg-hover);
	}

	.gd-trigger:focus-visible {
		border-color: var(--color-primary);
		box-shadow: 0 0 0 2px rgba(52, 211, 153, 0.15);
	}

	.gd-trigger.open {
		border-color: var(--color-primary);
	}

	.gd-trigger-text {
		flex: 1;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.gd-trigger-text.placeholder {
		color: var(--color-outline);
	}

	.gd-chevron {
		flex-shrink: 0;
		color: var(--color-outline);
		transition: transform 0.15s var(--ease-spring);
		display: flex;
		align-items: center;
	}

	.gd-chevron.open {
		transform: rotate(180deg);
	}

	/* ── Dropdown List ── */
	.gd-list {
		position: absolute;
		left: 0;
		right: 0;
		top: calc(100% + 4px);
		z-index: 1300;
		max-height: 220px;
		overflow-y: auto;
		margin: 0;
		padding: 4px;
		list-style: none;
		background: var(--glass-bg);
		backdrop-filter: var(--glass-blur, blur(24px) saturate(180%));
		-webkit-backdrop-filter: var(--glass-blur, blur(24px) saturate(180%));
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 10px;
		color: var(--color-on-surface);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.12),
			0 12px 40px rgba(0, 0, 0, 0.5),
			0 4px 12px rgba(0, 0, 0, 0.3);
		outline: none;

		/* Animate in */
		animation: gd-open 0.15s var(--ease-spring);
		transform-origin: top center;
	}

	.gd-list.flip-up {
		top: auto;
		bottom: calc(100% + 4px);
		transform-origin: bottom center;
	}

	@keyframes gd-open {
		from {
			opacity: 0;
			transform: scaleY(0.92) translateY(-2px);
		}
		to {
			opacity: 1;
			transform: scaleY(1) translateY(0);
		}
	}

	/* Light mode — lighter shadow, theme bg is set via CSS vars */
	:global([data-theme="light"]) .gd-list {
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.6),
			0 12px 40px rgba(0, 0, 0, 0.15),
			0 4px 12px rgba(0, 0, 0, 0.08);
	}

	/* ── Option ── */
	.gd-option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 7px 10px;
		border-radius: 6px;
		font-size: 0.85rem;
		color: var(--color-on-surface);
		cursor: pointer;
		transition: background 0.1s;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.gd-option.highlighted {
		background: rgba(255, 255, 255, 0.08);
	}

	:global([data-theme="light"]) .gd-option.highlighted {
		background: rgba(0, 0, 0, 0.06);
	}

	.gd-option.selected {
		color: var(--color-primary);
		font-weight: 500;
	}

	.gd-check {
		flex-shrink: 0;
		color: var(--color-primary);
		display: flex;
		align-items: center;
	}

	/* ── Scrollbar ── */
	.gd-list::-webkit-scrollbar {
		width: 4px;
	}
	.gd-list::-webkit-scrollbar-track {
		background: transparent;
	}
	.gd-list::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.12);
		border-radius: 2px;
	}
</style>
