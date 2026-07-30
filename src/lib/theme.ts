/**
 * Theme resolution and application.
 *
 * Two distinct shapes travel through here and must not be confused:
 *  - `ThemeMode` is what the user picked, and includes `system`.
 *  - `ResolvedScheme` is what actually gets painted, and never includes `system`.
 *
 * Keeping them as separate types is what makes it impossible to write `system`
 * into the `data-theme` attribute, whose contract admits only `light` or `dark`.
 */

import { getCurrentWindow } from '@tauri-apps/api/window';

export type ThemeMode = 'system' | 'light' | 'dark';
export type ResolvedScheme = 'light' | 'dark';

/** Mirrors `VALID_THEMES` in src-tauri/src/commands/settings.rs. */
const THEME_MODES: readonly ThemeMode[] = ['system', 'light', 'dark'];

const DARK_QUERY = '(prefers-color-scheme: dark)';

/**
 * Key of the synchronous pre-paint cache.
 *
 * SQLite holds the authoritative mode, but reading it means an async IPC round
 * trip that cannot finish before the first paint. This cache exists purely to
 * bridge that gap. It stores the *mode*, not the resolved scheme, so that a user
 * who chose `system` still gets re-resolved against the OS on the next launch.
 *
 * The inline script in src/app.html reads this same key before modules load and
 * must be kept in step with it.
 */
export const THEME_CACHE_KEY = 'vidbridge:theme-mode';

/**
 * Remember the mode for the next startup. Storage access can throw outright when
 * it is disabled, and a theme preference is never worth breaking a render over.
 */
export function cacheThemeMode(mode: ThemeMode): void {
	try {
		window.localStorage.setItem(THEME_CACHE_KEY, mode);
	} catch {
		// No cache means the next launch falls back to the OS preference — a brief
		// wrong theme at worst, so there is nothing useful to report here.
	}
}

/** Read the cached mode, normalising anything unexpected back to `system`. */
export function readCachedThemeMode(): ThemeMode {
	try {
		return normalizeThemeMode(window.localStorage.getItem(THEME_CACHE_KEY));
	} catch {
		return 'system';
	}
}

/**
 * Narrow an untrusted value — a database row, a localStorage entry, a mode a
 * future version removed — to a mode we know how to resolve. Anything
 * unrecognised becomes `system`, matching the Rust-side fallback, so the two
 * layers cannot disagree about what an unknown value means.
 */
export function normalizeThemeMode(value: unknown): ThemeMode {
	return THEME_MODES.includes(value as ThemeMode) ? (value as ThemeMode) : 'system';
}

/** The OS preference, defaulting to light where it cannot be read. */
export function systemScheme(): ResolvedScheme {
	if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'light';
	return window.matchMedia(DARK_QUERY).matches ? 'dark' : 'light';
}

/** Resolve any mode — valid or not — to the scheme that should be painted. */
export function resolveTheme(value: unknown): ResolvedScheme {
	const mode = normalizeThemeMode(value);
	return mode === 'system' ? systemScheme() : mode;
}

/** Write a resolved scheme to the document root. */
export function applyScheme(scheme: ResolvedScheme): void {
	if (typeof document === 'undefined') return;
	document.documentElement.dataset.theme = scheme;
}

/**
 * Align the native window appearance with the selected mode.
 *
 * CSS `color-scheme` reaches in-page form controls and scrollbars, but the macOS
 * `<select>` popup is a native NSMenu painted with the *window's* appearance,
 * which no stylesheet can touch. Without this, dropdowns stay dark while the rest
 * of a light theme renders correctly.
 *
 * Keyed on the mode, never on the resolved scheme, and `system` must pass `null`
 * rather than the resolved value. In a WKWebView `prefers-color-scheme` derives
 * from the window appearance, so forcing a concrete theme also overwrites the very
 * signal `system` resolves against — pinning the window to light makes `matchMedia`
 * report light forever, and `system` stops following the OS.
 *
 * Fire-and-forget: no Tauri host, or a missing capability, must not stop the
 * document attribute from having been applied.
 */
function applyNativeAppearance(mode: ThemeMode, onSettled: () => void): void {
	try {
		void getCurrentWindow()
			.setTheme(mode === 'system' ? null : mode)
			.then(onSettled)
			.catch(() => {});
	} catch {
		// getCurrentWindow() itself throws when there is no Tauri host.
	}
}

/**
 * Run `onChange` whenever the OS colour scheme flips. Returns a teardown.
 */
export function watchSystemScheme(onChange: (scheme: ResolvedScheme) => void): () => void {
	if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return () => {};

	const query = window.matchMedia(DARK_QUERY);
	const handler = (event: MediaQueryListEvent) => onChange(event.matches ? 'dark' : 'light');
	query.addEventListener('change', handler);
	return () => query.removeEventListener('change', handler);
}

/**
 * Apply `mode` and keep it current, returning a teardown.
 *
 * A fixed mode registers no listener at all, which is why an OS colour scheme
 * change cannot overwrite an explicit `light` or `dark` choice — the absence of a
 * subscription is the guarantee, not a check inside the handler.
 */
export function syncTheme(mode: unknown): () => void {
	const normalized = normalizeThemeMode(mode);

	// setTheme is async, so the synchronous resolve below can still observe the
	// appearance a previous explicit choice pinned the window to. Paint the best
	// value available now, then re-resolve once the native appearance has settled —
	// which is what makes switching back to `system` land on the real OS preference
	// instead of the theme that was pinned a moment ago.
	applyScheme(resolveTheme(normalized));

	// The settle callback outlives a fast mode switch: without this flag, the
	// callback from a superseded mode would re-resolve against its own stale mode
	// and repaint over the current one, leaving the final state dependent on the
	// order the IPC replies happen to arrive in.
	let cancelled = false;
	applyNativeAppearance(normalized, () => {
		if (!cancelled) applyScheme(resolveTheme(normalized));
	});

	// Only ever called with the authoritative value from the settings store, so
	// this is also the point at which the pre-paint cache is refreshed and any
	// divergence from SQLite is corrected.
	cacheThemeMode(normalized);

	const unwatch = normalized === 'system' ? watchSystemScheme(applyScheme) : () => {};
	return () => {
		cancelled = true;
		unwatch();
	};
}
