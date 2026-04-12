import { generateFullPalette, generateLightPalette, generateMonochromePalette, generateMonochromeLightPalette } from './engine';
import type { PaletteMode } from '$lib/stores/theme';

/**
 * Apply the generated palette as CSS custom properties on :root,
 * and update the body background gradient to match the seed color.
 */
export function applyTheme(seedHex: string, mode: 'dark' | 'light' = 'dark', paletteMode: PaletteMode = 'default') {
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

	// Dock-specific
	root.style.setProperty('--dock-bg', palette.surfaceContainer + 'd9'); // ~85% opacity
	root.style.setProperty('--dock-border', palette.outlineVariant + '33');
	root.style.setProperty('--dock-fg', palette.onSurface);

	// Dynamic background gradient — responds to seed color changes
	const g1 = palette.primary + '12';    // ~7% opacity
	const g2 = palette.secondary + '0f';  // ~6%
	const g3 = palette.tertiary + '0a';   // ~4%
	document.body.style.background = [
		`radial-gradient(ellipse 140% 90% at 15% 15%, ${g1} 0%, transparent 55%)`,
		`radial-gradient(ellipse 90% 130% at 85% 85%, ${g2} 0%, transparent 55%)`,
		`radial-gradient(ellipse 110% 110% at 50% 50%, ${g3} 0%, transparent 65%)`,
		palette.surface,
	].join(', ');
	document.body.style.backgroundAttachment = 'fixed';
}
