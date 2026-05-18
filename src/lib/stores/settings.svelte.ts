import { invoke } from '@tauri-apps/api/core';

export interface Settings {
    download_path: string;
    auto_organize: boolean;
    transcoding_preset: string;
    detect_clipboard: boolean;
}

class SettingsStore {
    settings = $state<Settings | null>(null);
    loading = $state(true);

    async load(retries = 10) {
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
