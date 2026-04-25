import { generateFullPalette, generateLightPalette, generateMonochromePalette, generateMonochromeLightPalette } from './engine';
import type { PaletteMode } from '$lib/stores/theme';

export interface BackgroundOptions {
	/** Data URL or path; null = no image, fallback to gradient. */
	image: string | null;
	/** 0..1 overlay strength; higher = more theme tint over the image. */
	overlay: number;
}

/**
 * Apply the generated palette as CSS custom properties on :root,
 * and update the body background gradient to match the seed color.
 *
 * When `bg.image` is set, it is composited under a theme-aware overlay so
 * the image stays visible but bright/dark areas conform to the active theme.
 */
export function applyTheme(
	seedHex: string,
	mode: 'dark' | 'light' = 'dark',
	paletteMode: PaletteMode = 'default',
	bg: BackgroundOptions = { image: null, overlay: 0.55 },
) {
	let palette;
	if (paletteMode === 'monochrome') {
		palette = mode === 'light'
			? generateMonochromeLightPalette(seedHex)
			: generateMonochromePalette(seedHex);
	} else {
		palette = mode === 'light'
			? generateLightPalette(seedHex)
			: generateFullPalette(seedHex);
	}

	const root = document.documentElement;

	root.setAttribute('data-theme', mode);

	root.style.setProperty('--color-primary', palette.primary);
	root.style.setProperty('--color-surface', palette.surface);
	root.style.setProperty('--color-secondary', palette.secondary);
	root.style.setProperty('--color-tertiary', palette.tertiary);
	root.style.setProperty('--color-on-primary', palette.onPrimary);
	root.style.setProperty('--color-on-surface', palette.onSurface);
	root.style.setProperty('--color-surface-container', palette.surfaceContainer);
	root.style.setProperty('--color-surface-container-high', palette.surfaceContainerHigh);
	root.style.setProperty('--color-outline', palette.outline);
	root.style.setProperty('--color-outline-variant', palette.outlineVariant);
	root.style.setProperty('--accent', palette.primary);

	// Dock-specific. Keep this in lockstep with the .glass-strong tier
	// (tier 3) — the dock floats over wallpaper and needs the same
	// readability profile as a modal. 85% was the old value; it made the
	// dock look like an opaque slab. ~58% (hex 94) lets the wallpaper
	// breathe through while strong blur still keeps icons legible.
	root.style.setProperty('--dock-bg', palette.surfaceContainer + '94');
	root.style.setProperty('--dock-border', palette.outlineVariant + '55');
	root.style.setProperty('--dock-fg', palette.onSurface);

	// Dynamic background gradient — responds to seed color changes
	const g1 = palette.primary + '12';    // ~7% opacity
	const g2 = palette.secondary + '0f';  // ~6%
	const g3 = palette.tertiary + '0a';   // ~4%

	const overlay = Math.min(0.95, Math.max(0, bg.overlay));

	if (bg.image) {
		// Theme-aware overlay: dark mode tints toward the surface (near-black),
		// light mode toward white. Saturated parts of the photo still bleed
		// through but desaturate enough to keep the UI legible. We use two
		// layers — a hard-stop wash for legibility plus the seed-color
		// gradient on top so brand colour still touches the chrome.
		const tintRgba =
			mode === 'dark'
				? `rgba(8,8,10,${overlay.toFixed(3)})`
				: `rgba(255,255,255,${overlay.toFixed(3)})`;
		// Light mode also boosts saturation gently via a subtle warm wash so
		// the picture stays vivid rather than washed out.
		const accentBoost =
			mode === 'light'
				? `, radial-gradient(ellipse 100% 100% at 50% 50%, ${palette.primary}10 0%, transparent 60%)`
				: '';
		document.body.style.background = [
			// Foreground gradients (very light) keep the seed colour signature.
			`radial-gradient(ellipse 140% 90% at 15% 15%, ${g1} 0%, transparent 55%)`,
			`radial-gradient(ellipse 90% 130% at 85% 85%, ${g2} 0%, transparent 55%)`,
			// Theme tint — strongest layer over the photo.
			`linear-gradient(${tintRgba}, ${tintRgba})${accentBoost}`,
			// The image itself, sized to cover the viewport.
			`url("${bg.image.replace(/"/g, '\\"')}") center / cover no-repeat fixed`,
			palette.surface,
		].join(', ');
	} else {
		document.body.style.background = [
			`radial-gradient(ellipse 140% 90% at 15% 15%, ${g1} 0%, transparent 55%)`,
			`radial-gradient(ellipse 90% 130% at 85% 85%, ${g2} 0%, transparent 55%)`,
			`radial-gradient(ellipse 110% 110% at 50% 50%, ${g3} 0%, transparent 65%)`,
			palette.surface,
		].join(', ');
	}
	document.body.style.backgroundAttachment = 'fixed';
}
