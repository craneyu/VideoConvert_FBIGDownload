<script lang="ts">
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { fade, fly } from "svelte/transition";

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

  function handleDetectClipboardChange(e: Event) {
    const target = e.target as HTMLInputElement;
    settingsStore.update('detect_clipboard', target.checked);
  }

  function handlePresetChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    settingsStore.update('transcoding_preset', target.value);
  }
</script>

<div class="min-h-screen bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-50 font-sans transition-colors duration-200 overflow-y-auto">
  <div class="max-w-3xl mx-auto p-10 space-y-10">
    <header class="flex justify-between items-end">
      <div>
        <h1 class="text-4xl font-black tracking-tight mb-2">軟體設定</h1>
        <p class="text-neutral-500 dark:text-neutral-400 font-medium">管理應用程式全域偏好設定與自動化規則</p>
      </div>
      <a href="/" class="px-6 py-3 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 hover:bg-neutral-100 dark:hover:bg-neutral-800 rounded-2xl font-bold transition-all text-sm shadow-sm flex items-center gap-2 group">
        <svg class="w-4 h-4 text-neutral-400 group-hover:text-blue-600 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path></svg>
        返回首頁
      </a>
    </header>

    {#if settingsStore.loading}
      <div class="flex justify-center p-20">
        <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600"></div>
      </div>
    {:else if settingsStore.settings}
      <div class="space-y-8">
        
        <!-- Download Settings -->
        <section class="bg-white dark:bg-neutral-900 rounded-3xl p-8 shadow-sm border border-neutral-200/80 dark:border-neutral-800 transition-all hover:shadow-md hover:border-neutral-300 dark:hover:border-neutral-700">
          <h2 class="text-xs font-black uppercase tracking-widest text-neutral-400 mb-8 flex items-center gap-2">
            <svg class="w-4 h-4 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path></svg>
            下載偏好設定
          </h2>
          
          <div class="space-y-6">
            <div class="space-y-2">
              <label class="text-xs font-bold text-neutral-500 dark:text-neutral-400 ml-1">預設下載路徑</label>
              <div class="flex gap-3">
                <input 
                  type="text" 
                  readonly
                  value={settingsStore.settings.download_path} 
                  class="flex-1 bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 rounded-2xl px-5 py-4 text-sm focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all outline-none"
                />
                <button 
                  onclick={selectDownloadPath}
                  class="px-8 py-4 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 rounded-2xl font-bold transition-all text-sm shadow-sm"
                >
                  瀏覽
                </button>
              </div>
            </div>

            <div class="pt-6 border-t border-neutral-100 dark:border-neutral-800/50">
              <label class="flex items-center gap-4 cursor-pointer p-4 hover:bg-neutral-50 dark:hover:bg-neutral-800/40 rounded-2xl transition-all group border border-transparent hover:border-neutral-100 dark:hover:border-neutral-800">
                <div class="relative flex items-center">
                  <input 
                    type="checkbox" 
                    checked={settingsStore.settings.auto_organize}
                    onchange={handleAutoOrganizeChange}
                    class="w-6 h-6 rounded-lg border-neutral-300 dark:border-neutral-700 text-blue-600 focus:ring-4 focus:ring-blue-500/10 transition-all cursor-pointer bg-white dark:bg-neutral-800 checked:bg-blue-600 active:scale-90"
                  />
                  {#if settingsStore.settings.auto_organize}
                    <div class="absolute inset-0 flex items-center justify-center pointer-events-none text-white scale-75" in:fly={{ y: 2, duration: 200 }}>
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"></path></svg>
                    </div>
                  {/if}
                </div>
                <div class="flex-1">
                  <div class="font-bold text-sm group-hover:text-blue-600 transition-colors flex items-center gap-2">
                    依來源自動分類
                    {#if settingsStore.settings.auto_organize}
                      <span class="px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/40 text-[10px] text-blue-600 dark:text-blue-400 font-black uppercase tracking-tighter" in:fade>Enabled</span>
                    {/if}
                  </div>
                  <div class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5 leading-relaxed">將下載檔案整理至對應來源名稱的子資料夾中 (例如: Downloads/VidBridge/Facebook/)</div>
                </div>
              </label>

              <label class="flex items-center gap-4 cursor-pointer p-4 mt-2 hover:bg-neutral-50 dark:hover:bg-neutral-800/40 rounded-2xl transition-all group border border-transparent hover:border-neutral-100 dark:hover:border-neutral-800">
                <div class="relative flex items-center">
                  <input 
                    type="checkbox" 
                    checked={settingsStore.settings.detect_clipboard}
                    onchange={handleDetectClipboardChange}
                    class="w-6 h-6 rounded-lg border-neutral-300 dark:border-neutral-700 text-blue-600 focus:ring-4 focus:ring-blue-500/10 transition-all cursor-pointer bg-white dark:bg-neutral-800 checked:bg-blue-600 active:scale-90"
                  />
                  {#if settingsStore.settings.detect_clipboard}
                    <div class="absolute inset-0 flex items-center justify-center pointer-events-none text-white scale-75" in:fly={{ y: 2, duration: 200 }}>
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"></path></svg>
                    </div>
                  {/if}
                </div>
                <div class="flex-1">
                  <div class="font-bold text-sm group-hover:text-blue-600 transition-colors flex items-center gap-2">
                    自動偵測剪貼簿
                    {#if settingsStore.settings.detect_clipboard}
                      <span class="px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/40 text-[10px] text-blue-600 dark:text-blue-400 font-black uppercase tracking-tighter" in:fade>Auto</span>
                    {/if}
                  </div>
                  <div class="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5 leading-relaxed">切換回應用程式時，自動辨識剪貼簿中的影片網址</div>
                </div>
              </label>
            </div>
          </div>
        </section>

        <!-- Transcoding Settings -->
        <section class="bg-white dark:bg-neutral-900 rounded-3xl p-8 shadow-sm border border-neutral-200/80 dark:border-neutral-800 transition-all hover:shadow-md hover:border-neutral-300 dark:hover:border-neutral-700">
          <h2 class="text-xs font-black uppercase tracking-widest text-neutral-400 mb-8 flex items-center gap-2">
            <svg class="w-4 h-4 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"></path></svg>
            轉檔品質配置
          </h2>
          
          <div class="space-y-6">
            <div class="space-y-2">
              <label class="text-xs font-bold text-neutral-500 dark:text-neutral-400 ml-1">預設轉檔品質 (Preset)</label>
              <div class="relative group">
                <select 
                  value={settingsStore.settings.transcoding_preset}
                  onchange={handlePresetChange}
                  class="w-full bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 rounded-2xl px-5 py-4 text-sm focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all outline-none appearance-none cursor-pointer group-hover:border-neutral-300 dark:group-hover:border-neutral-600"
                >
                  <option value="High">高品質 (High) - CRF 18</option>
                  <option value="Balanced">平衡 (Balanced) - CRF 23</option>
                  <option value="Fast">快速 (Fast) - CRF 28</option>
                </select>
                <div class="absolute right-5 top-1/2 -translate-y-1/2 pointer-events-none text-neutral-400">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
                </div>
              </div>
            </div>
          </div>
        </section>

      </div>
    {:else}
      <div class="text-center py-20 bg-red-50 dark:bg-red-900/10 rounded-3xl border border-red-100 dark:border-red-900/20">
        <p class="text-red-500 font-medium">無法載入設定資料，請檢查資料庫連線。</p>
        <button 
          onclick={() => settingsStore.load()}
          class="mt-4 px-6 py-2 bg-red-500 text-white rounded-xl font-bold hover:bg-red-600 transition-all shadow-lg shadow-red-500/20"
        >
          重新嘗試
        </button>
      </div>
    {/if}
  </div>
</div>
