import { invoke } from '@tauri-apps/api/core';

export interface Settings {
    download_path: string;
    auto_organize: boolean;
    transcoding_preset: string;
}

class SettingsStore {
    settings = $state<Settings | null>(null);
    loading = $state(true);

    async load() {
        this.loading = true;
        try {
            this.settings = await invoke<Settings>('get_settings');
        } catch (e) {
            console.error('Failed to load settings:', e);
        } finally {
            this.loading = false;
        }
    }

    async update(key: keyof Settings, value: any) {
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
