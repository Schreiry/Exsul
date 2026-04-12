import { Hsluv } from 'hsluv';

export interface TonalPalette {
	surface: string;
	primary: string;
	secondary: string;
	tertiary: string;
}

function hsluvToHex(h: number, s: number, l: number): string {
	const c = new Hsluv();
	c.hsluv_h = h;
	c.hsluv_s = s;
	c.hsluv_l = l;
	c.hsluvToHex();
	return c.hex;
}

function hexToHsluv(hex: string): [number, number, number] {
	const c = new Hsluv();
	c.hex = hex;
	c.hexToHsluv();
	return [c.hsluv_h, c.hsluv_s, c.hsluv_l];
}

/**
 * Generate 4 tonal variants (Surface, Primary, Secondary, Tertiary)
 * from a single seed hex color using HSLuv (perceptually uniform color space).
 */
export function generatePalette(seedHex: string): TonalPalette {
	const [h, s, l] = hexToHsluv(seedHex);

	return {
		primary: hsluvToHex(h, s, Math.min(l, 65)),
		surface: hsluvToHex(h, Math.max(s * 0.15, 5), 12),
		secondary: hsluvToHex((h + 30) % 360, Math.max(s * 0.7, 20), Math.min(l + 10, 75)),
		tertiary: hsluvToHex((h + 320) % 360, Math.max(s * 0.5, 15), Math.min(l + 20, 85)),
	};
}

/**
 * Generate extended dark-mode palette with light/dark text variants.
 */
export function generateFullPalette(seedHex: string) {
	const [h, s] = hexToHsluv(seedHex);
	const palette = generatePalette(seedHex);

	return {
		...palette,
		onPrimary: hsluvToHex(h, s * 0.3, 95),
		onSurface: hsluvToHex(h, s * 0.1, 90),
		surfaceContainer: hsluvToHex(h, Math.max(s * 0.12, 4), 18),
		surfaceContainerHigh: hsluvToHex(h, Math.max(s * 0.1, 3), 24),
		outline: hsluvToHex(h, Math.max(s * 0.2, 8), 40),
		outlineVariant: hsluvToHex(h, Math.max(s * 0.15, 5), 28),
	};
}

/**
 * Generate monochrome dark palette — strips saturation for austere, grayscale look.
 * Functional alert colors are handled separately in CSS tokens.
 */
export function generateMonochromePalette(_seedHex: string) {
	// Force h=0, s=0 for true greyscale — no hue leakage
	return {
		primary: hsluvToHex(0, 0, 65),
		secondary: hsluvToHex(0, 0, 55),
		tertiary: hsluvToHex(0, 0, 60),
		surface: hsluvToHex(0, 0, 8),
		onPrimary: hsluvToHex(0, 0, 95),
		onSurface: hsluvToHex(0, 0, 90),
		surfaceContainer: hsluvToHex(0, 0, 14),
		surfaceContainerHigh: hsluvToHex(0, 0, 20),
		outline: hsluvToHex(0, 0, 38),
		outlineVariant: hsluvToHex(0, 0, 25),
	};
}

/**
 * Generate monochrome light palette — desaturated, clean white surfaces.
 */
export function generateMonochromeLightPalette(_seedHex: string) {
	// Force h=0, s=0 for true greyscale — no hue leakage
	return {
		primary: hsluvToHex(0, 0, 45),
		secondary: hsluvToHex(0, 0, 40),
		tertiary: hsluvToHex(0, 0, 48),
		surface: hsluvToHex(0, 0, 97),
		onPrimary: hsluvToHex(0, 0, 98),
		onSurface: hsluvToHex(0, 0, 12),
		surfaceContainer: hsluvToHex(0, 0, 92),
		surfaceContainerHigh: hsluvToHex(0, 0, 86),
		outline: hsluvToHex(0, 0, 50),
		outlineVariant: hsluvToHex(0, 0, 78),
	};
}

/**
 * Generate extended light-mode palette — near-white surfaces, dark text.
 * Primary/secondary/tertiary hues are preserved from the seed.
 */
export function generateLightPalette(seedHex: string) {
	const [h, s, l] = hexToHsluv(seedHex);

	// Slightly deeper primary for better contrast on light bg
	const primary = hsluvToHex(h, s, Math.min(l, 55));
	const secondary = hsluvToHex((h + 30) % 360, Math.max(s * 0.7, 20), Math.min(l, 50));
	const tertiary = hsluvToHex((h + 320) % 360, Math.max(s * 0.5, 15), Math.min(l + 5, 60));

	return {
		primary,
		secondary,
		tertiary,
		surface: hsluvToHex(h, Math.max(s * 0.06, 2), 97),
		onPrimary: hsluvToHex(h, s * 0.15, 98),
		onSurface: hsluvToHex(h, Math.max(s * 0.15, 5), 12),
		surfaceContainer: hsluvToHex(h, Math.max(s * 0.10, 3), 92),
		surfaceContainerHigh: hsluvToHex(h, Math.max(s * 0.08, 2), 86),
		outline: hsluvToHex(h, Math.max(s * 0.25, 10), 50),
		outlineVariant: hsluvToHex(h, Math.max(s * 0.12, 4), 78),
	};
}
