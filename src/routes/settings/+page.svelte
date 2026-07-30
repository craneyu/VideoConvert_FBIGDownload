<script lang="ts">
  import { settingsStore, CONCURRENCY_RANGES } from "$lib/stores/settings.svelte";
  import type { Settings, VideoHandling } from "$lib/stores/settings.svelte";
  import type { ThemeMode } from "$lib/theme";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { fade, fly } from "svelte/transition";

  // Which original video codecs this machine reports it can decode. Needed so the
  // page can state what 'auto' currently resolves to — otherwise 'auto' tells the
  // user nothing about whether their downloads are kept or re-encoded.
  //
  // Stays `null` while the query is in flight and on failure, so the page never
  // states a capability it did not confirm.
  let decodableCodecs = $state<string[] | null>(null);
  $effect(() => {
    invoke<string[]>('decodable_video_codecs')
      .then((codecs) => { decodableCodecs = codecs; })
      .catch((e) => { console.error('Failed to query decodable codecs:', e); });
  });

  // AV1 is the codec that decides the outcome in practice: Facebook serves 1080p
  // Reels as AV1 only, and H.264 sources are remuxed under every policy anyway.
  const canKeepAv1 = $derived(decodableCodecs === null ? null : decodableCodecs.includes('av1'));

  const VIDEO_HANDLING_OPTIONS: Array<{ value: VideoHandling; label: string }> = [
    { value: 'auto', label: '自動判斷' },
    { value: 'original', label: '保留原檔' },
    { value: 'compat', label: '相容優先' }
  ];

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

  type ConcurrencyKey = keyof typeof CONCURRENCY_RANGES;

  /// Write a concurrency limit only when the field holds a usable value.
  ///
  /// An empty or out-of-range field is reverted to the stored value rather than
  /// sent. The backend would accept the write and then fall back to its default
  /// when reading, which the user reads as the setting having failed to save.
  function handleConcurrencyChange(key: ConcurrencyKey) {
    return (e: Event) => {
      const target = e.target as HTMLInputElement;
      const { min, max } = CONCURRENCY_RANGES[key];
      const parsed = Number.parseInt(target.value, 10);

      if (!Number.isInteger(parsed) || parsed < min || parsed > max) {
        target.value = String(settingsStore.settings?.[key] ?? min);
        return;
      }
      settingsStore.update(key as keyof Settings, parsed);
    };
  }

  // "跟隨系統" is first because it is the default, and the only option whose
  // result depends on something outside the app.
  const THEME_OPTIONS: Array<{ value: ThemeMode; label: string }> = [
    { value: 'system', label: '跟隨系統' },
    { value: 'light', label: '淺色' },
    { value: 'dark', label: '深色' }
  ];
</script>

<!--
  Spacing is deliberately tight: the default window is 800x600, and the previous
  values pushed the transcoding section below the fold so the page always needed
  scrolling. Every setting now fits on one screen at that size.
-->
<div class="min-h-screen bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-50 font-sans transition-colors duration-200 overflow-y-auto">
  <div class="max-w-3xl mx-auto p-6 space-y-5">
    <header class="flex justify-between items-end">
      <div>
        <h1 class="text-2xl font-black tracking-tight mb-1">軟體設定</h1>
        <p class="text-xs text-neutral-500 dark:text-neutral-400 font-medium">管理應用程式全域偏好設定與自動化規則</p>
      </div>
      <a href="/" class="px-4 py-2 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 hover:bg-neutral-100 dark:hover:bg-neutral-800 rounded-xl font-bold transition-all text-sm shadow-sm flex items-center gap-2 group">
        <svg class="w-4 h-4 text-neutral-400 group-hover:text-blue-600 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path></svg>
        返回首頁
      </a>
    </header>

    {#if settingsStore.loading}
      <div class="flex justify-center p-10">
        <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600"></div>
      </div>
    {:else if settingsStore.settings}
      <div class="space-y-4">

        <!--
          The download card and the concurrency card share a row. The download card
          is the tall one, so putting concurrency beside it costs almost no extra
          height — which is what keeps every setting on one 800x600 screen.
        -->
        <div class="grid grid-cols-3 gap-4 items-start">

        <!-- Download Settings -->
        <section class="col-span-2 bg-white dark:bg-neutral-900 rounded-2xl p-5 shadow-sm border border-neutral-200/80 dark:border-neutral-800 transition-all hover:shadow-md hover:border-neutral-300 dark:hover:border-neutral-700">
          <h2 class="text-xs font-black uppercase tracking-widest text-neutral-400 mb-4 flex items-center gap-2">
            <svg class="w-4 h-4 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path></svg>
            下載偏好設定
          </h2>

          <div class="space-y-4">
            <div class="space-y-1.5">
              <label class="text-xs font-bold text-neutral-500 dark:text-neutral-400 ml-1">預設下載路徑</label>
              <div class="flex gap-2">
                <input
                  type="text"
                  readonly
                  value={settingsStore.settings.download_path}
                  class="flex-1 bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 rounded-xl px-4 py-2.5 text-sm focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all outline-none"
                />
                <button
                  onclick={selectDownloadPath}
                  class="px-6 py-2.5 bg-neutral-100 hover:bg-neutral-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 rounded-xl font-bold transition-all text-sm shadow-sm"
                >
                  瀏覽
                </button>
              </div>
            </div>

            <div class="pt-3 border-t border-neutral-100 dark:border-neutral-800/50">
              <label class="flex items-center gap-3 cursor-pointer p-2.5 hover:bg-neutral-50 dark:hover:bg-neutral-800/40 rounded-xl transition-all group border border-transparent hover:border-neutral-100 dark:hover:border-neutral-800">
                <div class="relative flex items-center">
                  <input
                    type="checkbox"
                    checked={settingsStore.settings.auto_organize}
                    onchange={handleAutoOrganizeChange}
                    class="w-5 h-5 rounded-md border-neutral-300 dark:border-neutral-700 text-blue-600 focus:ring-4 focus:ring-blue-500/10 transition-all cursor-pointer bg-white dark:bg-neutral-800 checked:bg-blue-600 active:scale-90"
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
                  <div class="text-xs text-neutral-500 dark:text-neutral-400 leading-snug">將下載檔案整理至對應來源名稱的子資料夾中 (例如: Downloads/VidBridge/Facebook/)</div>
                </div>
              </label>

              <label class="flex items-center gap-3 cursor-pointer p-2.5 mt-1 hover:bg-neutral-50 dark:hover:bg-neutral-800/40 rounded-xl transition-all group border border-transparent hover:border-neutral-100 dark:hover:border-neutral-800">
                <div class="relative flex items-center">
                  <input
                    type="checkbox"
                    checked={settingsStore.settings.detect_clipboard}
                    onchange={handleDetectClipboardChange}
                    class="w-5 h-5 rounded-md border-neutral-300 dark:border-neutral-700 text-blue-600 focus:ring-4 focus:ring-blue-500/10 transition-all cursor-pointer bg-white dark:bg-neutral-800 checked:bg-blue-600 active:scale-90"
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
                  <div class="text-xs text-neutral-500 dark:text-neutral-400 leading-snug">切換回應用程式時，自動辨識剪貼簿中的影片網址</div>
                </div>
              </label>
            </div>
          </div>
        </section>

        <!--
          Concurrency gets its own card so the two limits sit together: they are one
          decision with two halves, and the download card would otherwise grow tall
          enough to push this row past a single 800x600 screen — which the spacing
          note at the top of this file exists to prevent.
        -->
        <section class="bg-white dark:bg-neutral-900 rounded-2xl p-5 shadow-sm border border-neutral-200/80 dark:border-neutral-800 transition-all hover:shadow-md hover:border-neutral-300 dark:hover:border-neutral-700">
          <h2 class="text-xs font-black uppercase tracking-widest text-neutral-400 mb-4 flex items-center gap-2">
            <svg class="w-4 h-4 text-emerald-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h7"></path></svg>
            並行處理
          </h2>

          <div class="space-y-3">
            <div class="flex items-center gap-3">
              <input
                id="network-concurrency"
                type="number"
                min={CONCURRENCY_RANGES.max_network_concurrency.min}
                max={CONCURRENCY_RANGES.max_network_concurrency.max}
                value={settingsStore.settings.max_network_concurrency}
                onchange={handleConcurrencyChange('max_network_concurrency')}
                class="w-16 bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 rounded-xl px-3 py-2 text-sm focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all outline-none"
              />
              <div class="min-w-0">
                <label for="network-concurrency" class="block text-xs font-bold">同時下載</label>
                <p class="text-[10px] text-neutral-500 dark:text-neutral-400 leading-tight">1–8・立即生效</p>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <input
                id="cpu-concurrency"
                type="number"
                min={CONCURRENCY_RANGES.max_cpu_concurrency.min}
                max={CONCURRENCY_RANGES.max_cpu_concurrency.max}
                value={settingsStore.settings.max_cpu_concurrency}
                onchange={handleConcurrencyChange('max_cpu_concurrency')}
                class="w-16 bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 rounded-xl px-3 py-2 text-sm focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all outline-none"
              />
              <div class="min-w-0">
                <label for="cpu-concurrency" class="block text-xs font-bold">同時編碼</label>
                <p class="text-[10px] text-amber-600 dark:text-amber-500 leading-tight">1–2・變更後需重啟</p>
              </div>
            </div>

            <p class="text-[10px] text-neutral-500 dark:text-neutral-400 leading-snug pt-1 border-t border-neutral-100 dark:border-neutral-800/50">
              編碼名額由下載後處理與轉檔<span class="font-bold">共用</span>。設為 2 是為了<span class="font-bold">讓程式保持回應</span>，不會更快 —— 編碼本來就吃滿所有核心。等待編碼的下載不佔用下載名額。
            </p>
          </div>
        </section>

        </div>

        <!--
          Transcoding and appearance sit side by side rather than stacked. Stacking
          a third section pushed the page past one 800x600 screen, which the spacing
          note above exists to prevent.
        -->
        <div class="grid grid-cols-2 gap-4">

        <!-- Transcoding Settings -->
        <section class="bg-white dark:bg-neutral-900 rounded-2xl p-5 shadow-sm border border-neutral-200/80 dark:border-neutral-800 transition-all hover:shadow-md hover:border-neutral-300 dark:hover:border-neutral-700">
          <h2 class="text-xs font-black uppercase tracking-widest text-neutral-400 mb-4 flex items-center gap-2">
            <svg class="w-4 h-4 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"></path></svg>
            轉檔品質配置
          </h2>

          <div class="space-y-1.5">
            <label class="text-xs font-bold text-neutral-500 dark:text-neutral-400 ml-1">預設轉檔品質 (Preset)</label>
            <div class="relative group">
              <select
                value={settingsStore.settings.transcoding_preset}
                onchange={handlePresetChange}
                class="w-full bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 rounded-xl px-4 py-2.5 text-sm focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all outline-none appearance-none cursor-pointer group-hover:border-neutral-300 dark:group-hover:border-neutral-600"
              >
                <option value="High">高品質 (High) - CRF 18</option>
                <option value="Balanced">平衡 (Balanced) - CRF 23</option>
                <option value="Fast">快速 (Fast) - CRF 28</option>
              </select>
              <div class="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-neutral-400">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
              </div>
            </div>
          </div>

          <!--
            Video handling belongs with encoding decisions, and this card is the short
            one in its row — putting it here costs no extra page height, which the
            note at the top of this file exists to protect.
          -->
          <div class="space-y-1.5 mt-4 pt-3 border-t border-neutral-100 dark:border-neutral-800/50">
            <p class="text-xs font-bold text-neutral-500 dark:text-neutral-400 ml-1">下載影片的處理方式</p>
            <div class="flex gap-1 p-1 bg-neutral-100 dark:bg-neutral-800/60 rounded-xl">
              {#each VIDEO_HANDLING_OPTIONS as option (option.value)}
                <button
                  onclick={() => settingsStore.update('download_video_handling', option.value)}
                  aria-pressed={settingsStore.settings.download_video_handling === option.value}
                  class="flex-1 px-1.5 py-1.5 rounded-lg text-[11px] font-bold transition-all {settingsStore.settings.download_video_handling === option.value
                    ? 'bg-white dark:bg-neutral-700 text-blue-600 dark:text-blue-400 shadow-sm'
                    : 'text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'}"
                >
                  {option.label}
                </button>
              {/each}
            </div>
            <p class="text-[10px] text-neutral-500 dark:text-neutral-400 leading-snug ml-1">
              {#if settingsStore.settings.download_video_handling === 'auto'}
                {#if canKeepAv1 === null}
                  正在偵測本機的解碼能力…
                {:else if canKeepAv1}
                  <span class="font-bold text-emerald-600 dark:text-emerald-500">本機可解碼 AV1 → 保留原始畫質</span>（免重編、檔案更小、幾乎瞬間完成）
                {:else}
                  <span class="font-bold">未能確認本機可解碼 AV1 → 重新編碼為 H.264</span>
                {/if}
              {:else if settingsStore.settings.download_video_handling === 'original'}
                一律保留原始視訊串流，不重新編碼。
              {:else}
                一律重新編碼為 H.264，維持最大相容性。
              {/if}
            </p>
            <p class="text-[10px] text-amber-600 dark:text-amber-500 leading-snug ml-1">
              保留原檔畫質更好、檔案更小，但<span class="font-bold">可能在不支援該編碼的裝置上無法播放</span> —— 偵測只反映這台機器。
            </p>
          </div>

        </section>

        <!-- Appearance -->
        <section class="bg-white dark:bg-neutral-900 rounded-2xl p-5 shadow-sm border border-neutral-200/80 dark:border-neutral-800 transition-all hover:shadow-md hover:border-neutral-300 dark:hover:border-neutral-700">
          <h2 class="text-xs font-black uppercase tracking-widest text-neutral-400 mb-4 flex items-center gap-2">
            <svg class="w-4 h-4 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828L13 15.657"></path></svg>
            外觀
          </h2>

          <div class="space-y-1.5">
            <p class="text-xs font-bold text-neutral-500 dark:text-neutral-400 ml-1">介面主題</p>
            <div class="flex gap-1 p-1 bg-neutral-100 dark:bg-neutral-800/60 rounded-xl">
              {#each THEME_OPTIONS as option (option.value)}
                <button
                  onclick={() => settingsStore.update('theme', option.value)}
                  aria-pressed={settingsStore.settings.theme === option.value}
                  class="flex-1 px-2 py-1.5 rounded-lg text-xs font-bold transition-all {settingsStore.settings.theme === option.value
                    ? 'bg-white dark:bg-neutral-700 text-blue-600 dark:text-blue-400 shadow-sm'
                    : 'text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'}"
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </div>
        </section>

        </div>

      </div>
    {:else}
      <div class="text-center py-10 bg-red-50 dark:bg-red-900/10 rounded-2xl border border-red-100 dark:border-red-900/20">
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
