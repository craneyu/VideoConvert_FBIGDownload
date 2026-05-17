<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { open } from "@tauri-apps/plugin-dialog";

  async function selectDownloadPath() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    
    if (selected && typeof selected === 'string') {
      settingsStore.update('download_path', selected);
    }
  }

  function handleAutoOrganizeChange(e: Event) {
    const target = e.target as HTMLInputElement;
    settingsStore.update('auto_organize', target.checked);
  }

  function handlePresetChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    settingsStore.update('transcoding_preset', target.value);
  }
</script>

<div class="max-w-2xl mx-auto p-8 space-y-8">
  <header class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-black tracking-tight">設定</h1>
      <p class="text-neutral-500 mt-2">管理應用程式全域偏好設定</p>
    </div>
    <a href="/" class="px-6 py-3 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 rounded-xl font-bold transition-all text-sm">
      返回首頁
    </a>
  </header>

  {#if settingsStore.loading}
    <div class="flex justify-center p-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
    </div>
  {:else if settingsStore.settings}
    <div class="space-y-6">
      
      <!-- Download Settings -->
      <section class="bg-white dark:bg-neutral-900 rounded-2xl p-6 shadow-sm border border-neutral-100 dark:border-neutral-800">
        <h2 class="text-xl font-bold mb-4">下載設定</h2>
        
        <div class="space-y-4">
          <div class="space-y-1.5">
            <label class="text-sm font-bold text-neutral-600 dark:text-neutral-400">預設下載路徑</label>
            <div class="flex gap-2">
              <input 
                type="text" 
                readonly
                value={settingsStore.settings.download_path} 
                class="flex-1 bg-neutral-50 dark:bg-neutral-800 border-none rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-blue-500 transition-all outline-none"
              />
              <button 
                onclick={selectDownloadPath}
                class="px-6 py-3 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 rounded-xl font-bold transition-all text-sm"
              >
                瀏覽
              </button>
            </div>
          </div>

          <label class="flex items-center gap-3 cursor-pointer p-2 hover:bg-neutral-50 dark:hover:bg-neutral-800 rounded-xl transition-colors">
            <input 
              type="checkbox" 
              checked={settingsStore.settings.auto_organize}
              onchange={handleAutoOrganizeChange}
              class="w-5 h-5 rounded border-neutral-300 text-blue-600 focus:ring-blue-500"
            />
            <div>
              <div class="font-bold text-sm">依來源自動分類</div>
              <div class="text-xs text-neutral-500">將下載檔案整理至對應來源名稱的子資料夾中</div>
            </div>
          </label>
        </div>
      </section>

      <!-- Transcoding Settings -->
      <section class="bg-white dark:bg-neutral-900 rounded-2xl p-6 shadow-sm border border-neutral-100 dark:border-neutral-800">
        <h2 class="text-xl font-bold mb-4">轉檔設定</h2>
        
        <div class="space-y-4">
          <div class="space-y-1.5">
            <label class="text-sm font-bold text-neutral-600 dark:text-neutral-400">預設轉檔品質 (Preset)</label>
            <select 
              value={settingsStore.settings.transcoding_preset}
              onchange={handlePresetChange}
              class="w-full bg-neutral-50 dark:bg-neutral-800 border-none rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-blue-500 transition-all outline-none"
            >
              <option value="High">高品質 (High) - CRF 18</option>
              <option value="Balanced">平衡 (Balanced) - CRF 23</option>
              <option value="Fast">快速 (Fast) - CRF 28</option>
            </select>
          </div>
        </div>
      </section>

    </div>
  {/if}
</div>