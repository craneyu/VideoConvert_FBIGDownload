import { invoke } from '@tauri-apps/api/core';
import type { ThemeMode } from '$lib/theme';

export interface Settings {
    download_path: string;
    auto_organize: boolean;
    transcoding_preset: string;
    detect_clipboard: boolean;
    // The backend narrows this to the same three values before it ever reaches
    // here, so the union is a faithful description rather than an optimistic one.
    theme: ThemeMode;
    // How many downloads run their network phase at once. Read live by the
    // download queue, so a change applies without a restart.
    max_network_concurrency: number;
    // How many re-encodes run at once, shared by the download and transcoding
    // pipelines. The permit pool is built at startup, so a change applies on the
    // next launch.
    max_cpu_concurrency: number;
    // Whether a downloaded video keeps its original stream or is re-encoded for
    // compatibility. 'auto' is resolved per download from the platform's decode
    // capability and is stored as 'auto', never as the resolved answer.
    download_video_handling: VideoHandling;
}

export type VideoHandling = 'auto' | 'original' | 'compat';

// Mirrors the ranges the backend accepts when merging stored settings. The
// backend is authoritative — it falls back to the default for anything outside
// these — and these values exist so the inputs cannot offer a value that would be
// silently discarded.
export const CONCURRENCY_RANGES = {
    max_network_concurrency: { min: 1, max: 8 },
    max_cpu_concurrency: { min: 1, max: 2 }
} as const;

class SettingsStore {
    settings = $state<Settings | null>(null);
    loading = $state(true);

    // Explicit return type: the method recurses on retry, so TypeScript cannot
    // infer it and reports an implicit-any error without this annotation.
    async load(retries = 10): Promise<void> {
        this.loading = true;
        try {
            this.settings = await invoke<Settings>('get_settings');
        } catch (e) {
            console.error('Failed to load settings:', e);
            if (retries > 0 && String(e).includes('Database not loaded')) {
                console.log(`Database not ready, retrying... (${retries} left)`);
                await new Promise(resolve => setTimeout(resolve, 1000));
                return this.load(retries - 1);
            }
        } finally {
            if (this.settings || retries === 0) {
                this.loading = false;
            }
        }
    }

    // Generic over the key so each setting only accepts its own value type — an
    // unrecognised theme literal is then a compile error here rather than a value
    // the backend has to quietly discard.
    async update<K extends keyof Settings>(key: K, value: Settings[K]) {
        if (!this.settings) return;

        // Optimistic update
        const originalValue = this.settings[key];
        (this.settings as any)[key] = value;

        try {
            await invoke('update_setting', { key, value });
        } catch (e) {
            console.error(`Failed to update setting ${key}:`, e);
            // Revert on failure
            (this.settings as any)[key] = originalValue;
        }
    }
}

export const settingsStore = new SettingsStore();
