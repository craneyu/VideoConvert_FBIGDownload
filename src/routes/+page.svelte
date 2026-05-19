<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Database from "@tauri-apps/plugin-sql";
  import { downloadDir } from "@tauri-apps/api/path";
  import { open } from "@tauri-apps/plugin-dialog";
  import { check } from "@tauri-apps/plugin-updater";
  import { onMount } from "svelte";
  import { slide, fade, fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { settingsStore } from "$lib/stores/settings.svelte";

  // --- Types ---
  interface DownloadTask {
    id: string;
    url: string;
    title: string;
    status: 'pending' | 'fetching' | 'downloading' | 'completed' | 'failed';
    progress: number;
    speed: string;
    source: 'Facebook' | 'Instagram' | 'Unknown';
    dbId?: number;
  }

  interface TranscodeTask {
    id: string;
    inputPath: string;
    fileName: string;
    status: 'pending' | 'processing' | 'completed' | 'failed';
    progress: number;
    time: string;
    outputPath?: string;
  }

  // --- App State ---
  let activeTab = $state("download"); // "download" | "transcode" | "history"
  let db: any;
  let urlInput = $state("");
  let detectedUrl = $state(""); // New: Store detected clipboard URL
  let lastCheckedClipboard = ""; // New: Prevent duplicate detection

  // Regex Patterns for FB/IG
  const FB_REGEX = /https?:\/\/(www\.)?(facebook\.com|fb\.watch)\/.+/;
  const IG_REGEX = /https?:\/\/(www\.)?instagram\.com\/(p|reels|reel)\/.+/;

  // Queue State
  let downloadTasks = $state<DownloadTask[]>([]);
  let transcodeTasks = $state<TranscodeTask[]>([]);
  let showAdvanced = $state(false);
  let history = $state<any[]>([]);


  // Config
  const MAX_CONCURRENT_DOWNLOADS = 2;
  let globalOptions = $state({
    preset: "balanced",
    resolution: "original",
    codec: "h264"
  });

  let settingsApplied = false;
  $effect(() => {
    if (settingsStore.settings && !settingsApplied) {
      globalOptions.preset = settingsStore.settings.transcoding_preset.toLowerCase();
      settingsApplied = true;
    }
  });

  onMount(async () => {
    console.log("onMount triggered, loading database...");
    try {
      db = await Database.load("sqlite:vidbridge.db");
      console.log("Database loaded successfully:", db);
      await loadHistory();
    } catch (e) {
      console.error("CRITICAL: Failed to load database:", e);
      alert("資料庫連線失敗，請檢查權限或重新啟動程式。");
    }

    await checkUpdate();

    // Check dependencies and auto-install if needed
    try {
      const result: any = await invoke("check_dependencies");
      if (result && result.dependencies) {
        const missing = result.dependencies.filter((d: any) => !d.installed);
        if (missing.length > 0) {
          const toolNames = missing.map((d: any) => d.name);
          const userConfirm = confirm(
            `偵測到您的系統 (${result.platform}) 缺少以下必要組件：\n${toolNames.join(", ")}\n\n是否要自動安裝？`
          );
          if (userConfirm) {
            const installResults: string[] = await invoke("install_dependencies", { tools: toolNames });
            const summary = installResults.join("\n");
            alert(`安裝結果：\n${summary}`);
          } else {
            alert(`請手動安裝以下組件：${toolNames.join(", ")}`);
          }
        } else {
          // All installed, check for updates for yt-dlp (most frequently updated)
          const ytdlp = result.dependencies.find((d: any) => d.name === "yt-dlp");
          if (ytdlp && ytdlp.installed) {
            // Silently attempt to update yt-dlp in background
            invoke("install_dependencies", { tools: ["yt-dlp"] }).catch((e) => console.error("Background yt-dlp update failed:", e));
          }
        }
      }
    } catch (e) {
      console.error("Dependency check failed:", e);
    }

    // Progress Listeners
    const unlistenDl = await listen("download-progress", (event: any) => {
      const task = downloadTasks.find(t => t.id === event.payload.id || (t.dbId && t.dbId.toString() === event.payload.id));
      if (task) {
        task.progress = event.payload.progress;
        task.speed = event.payload.speed;
        if (task.status === 'fetching') task.status = 'downloading';
      }
    });

    const unlistenTr = await listen("transcode-progress", (event: any) => {
      const task = transcodeTasks.find(t => t.id === event.payload.id);
      if (task) {
        task.progress = event.payload.progress;
        task.time = event.payload.time;
      }
    });
const unlistenDrop = await listen("tauri://drag-drop", (event: any) => {
  console.log("Drag-drop event received:", event);
  // In Tauri 2, payload might be an object containing 'paths' or just the paths array
  const paths = Array.isArray(event.payload) ? event.payload : (event.payload?.paths || []);

  if (activeTab === "transcode" && paths.length > 0) {
    for (const path of paths) {
      if (typeof path === 'string' && path.match(/\.(mp4|avi|mov|mkv)$/i)) {
        addTranscodeTask(path);
      }
    }
  }
});

    return () => {
      unlistenDl();
      unlistenTr();
      unlistenDrop();
    };
  });

  // --- Clipboard Detection Logic ---
  $effect(() => {
    const handleFocus = async () => {
      if (settingsStore.settings?.detect_clipboard && activeTab === 'download') {
        try {
          const text = await invoke<string | null>("read_clipboard_text");
          if (text && text !== lastCheckedClipboard && text !== urlInput) {
            lastCheckedClipboard = text;
            if (FB_REGEX.test(text) || IG_REGEX.test(text)) {
              console.log("Detected video URL in clipboard:", text);
              detectedUrl = text;
            } else {
              detectedUrl = "";
            }
          }
        } catch (e) {
          console.error("Clipboard access error:", e);
        }
      }
    };

    window.addEventListener("focus", handleFocus);
    // Trigger once on effect run if focused
    if (document.hasFocus()) handleFocus();

    return () => window.removeEventListener("focus", handleFocus);
  });

  async function loadHistory() {
    if (db) {
      history = await db.select("SELECT * FROM download_history ORDER BY created_at DESC");
    }
  }

  async function checkUpdate() {
    try {
      const update = await check();
      if (update) {
        console.log(`發現新版本: ${update.version}, 日期: ${update.date}`);
        let message = `發現新版本 v${update.version}！\n\n更新內容:\n${update.body || "無描述"}\n\n是否現在下載並安裝？`;
        if (confirm(message)) {
          await update.downloadAndInstall();
        }
      }
    } catch (e) {
      // Ignore updater errors for now (e.g. repo not set up yet)
    }
  }

  // --- Download Queue Logic ---
  async function addDownloadTask() {
    if (!urlInput) return;
    const url = urlInput;
    urlInput = "";

    const source = url.includes("facebook.com") || url.includes("fb.watch") ? "Facebook" : 
                   url.includes("instagram.com") ? "Instagram" : "Unknown";

    const newTask: DownloadTask = {
      id: Math.random().toString(36).substring(7),
      url,
      title: "正在解析影片資訊...",
      status: 'pending',
      progress: 0,
      speed: "",
      source
    };

    downloadTasks = [...downloadTasks, newTask];
    processQueue();
  }

  async function processQueue() {
    if (!db) {
      console.log("Database not ready, retrying in 500ms...");
      setTimeout(processQueue, 500);
      return;
    }
    const activeCount = downloadTasks.filter(t => t.status === 'fetching' || t.status === 'downloading').length;
    if (activeCount >= MAX_CONCURRENT_DOWNLOADS) return;

    const nextTask = downloadTasks.find(t => t.status === 'pending');
    if (!nextTask) return;

    nextTask.status = 'fetching';
    
    try {
      console.log("Starting task:", nextTask.url);
      const fetchedTitle = await invoke("fetch_video_info", { url: nextTask.url });
      nextTask.title = String(fetchedTitle);
      console.log("Title fetched:", nextTask.title);
      
      const res: any = await db.execute(
        "INSERT INTO download_history (url, title, status, source) VALUES (?, ?, ?, ?)",
        [nextTask.url, nextTask.title, "downloading", nextTask.source]
      );
      
      nextTask.dbId = res.lastInsertId;
      console.log("Recorded to DB, ID:", nextTask.dbId);
      await loadHistory();

      const dlDir = settingsStore.settings?.download_path || await downloadDir();
      // Important: id must be a string for Rust's String type
      const taskId = String(nextTask.dbId);
      
      console.log("Invoking download_video with:", { id: taskId, url: nextTask.url, downloadDir: dlDir, source: nextTask.source });
      
      const finalPath = await invoke("download_video", { 
        id: taskId, 
        url: nextTask.url, 
        downloadDir: dlDir,
        source: nextTask.source,
        autoOrganize: settingsStore.settings?.auto_organize ?? false
      });

      console.log("Download finished, path:", finalPath);
      // 4. Update status
      nextTask.status = 'completed';
      nextTask.progress = 100;
      (nextTask as any).file_path = finalPath; // Store the path for the 'Open Folder' button
      await db.execute(
        "UPDATE download_history SET status = ?, file_path = ? WHERE id = ?",
        ["completed", finalPath, nextTask.dbId]
      );

      await loadHistory();
    } catch (e) {
      nextTask.status = 'failed';
      console.error("Task error:", e);
      alert(`任務失敗: ${e}`);
    } finally {
      processQueue();
    }
  }

  // --- Transcode Logic ---
  async function selectFiles() {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: 'Video', extensions: ['mp4', 'avi', 'mov', 'mkv'] }]
      });
      
      console.log("Files selected:", selected);
      
      if (Array.isArray(selected)) {
        selected.forEach((file: any) => {
          // Tauri 2 open() returns an array of objects or strings depending on configuration
          const path = typeof file === 'string' ? file : file.path;
          if (path) addTranscodeTask(path);
        });
      } else if (selected) {
        const path = typeof selected === 'string' ? selected : (selected as any).path;
        if (path) addTranscodeTask(path);
      }
    } catch (e) {
      console.error("File selection error:", e);
    }
  }

  function addTranscodeTask(path: string) {
    console.log("Adding transcode task for path:", path);
    const fileName = path.split('/').pop() || path;
    const newTask: TranscodeTask = {
      id: Math.random().toString(36).substring(7),
      inputPath: path,
      fileName,
      status: 'pending',
      progress: 0,
      time: '00:00:00'
    };
    
    // Explicitly re-assign the array to trigger Svelte reactivity
    transcodeTasks = [newTask, ...transcodeTasks];
    console.log("Transcode tasks updated, current count:", transcodeTasks.length);
  }

  async function startTranscode(task: TranscodeTask) {
    if (task.status === 'processing') return;
    task.status = 'processing';
    console.log("Starting transcoding for task:", task.id, task.inputPath);

    try {
      const dlDir = settingsStore.settings?.download_path || await downloadDir();
      // Ensure the base path ends with a slash or add one
      const basePath = dlDir.endsWith('/') ? dlDir : `${dlDir}/`;
      const transcodedDir = `${basePath}VidBridge/Transcoded`;
      const outputPath = `${transcodedDir}/${task.fileName.replace(/\.[^/.]+$/, "")}_converted.mp4`;
      
      console.log("Output path will be:", outputPath);

      const result = await invoke("transcode_video", {
        id: task.id,
        inputPath: task.inputPath,
        outputPath,
        options: $state.snapshot(globalOptions)
      });

      console.log("Transcoding finished successfully:", result);
      task.status = 'completed';
      task.progress = 100;
      task.outputPath = outputPath;
    } catch (e) {
      console.error("Transcode error:", e);
      task.status = 'failed';
      alert(`轉檔失敗: ${e}`);
    }
  }

  async function openFile(path: string) {
    if (path) await invoke("open_folder", { path });
  }

  async function deleteHistoryRecord(id: number) {
    if (db && confirm("確定要刪除此筆紀錄嗎？（不會刪除實際檔案）")) {
      await db.execute("DELETE FROM download_history WHERE id = ?", [id]);
      await loadHistory();
    }
  }

  async function retryFromHistory(item: any) {
    urlInput = item.url;
    addDownloadTask();
  }

  function setPreset(p: string) {
    globalOptions.preset = p;
    globalOptions.codec = "h264";
    globalOptions.resolution = p === "fast" ? "720" : "original";
  }
</script>

<div class="flex h-screen bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-50 overflow-hidden font-sans">
  <!-- Sidebar -->
  <aside class="w-64 bg-neutral-100/50 dark:bg-neutral-900/50 border-r border-neutral-200 dark:border-neutral-800 flex flex-col p-6">
    <div class="flex items-center gap-3 mb-10 px-2">
      <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center shadow-lg shadow-blue-500/20">
        <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path></svg>
      </div>
      <span class="font-bold text-xl tracking-tight">VidBridge</span>
    </div>

    <nav class="space-y-1 flex-1">
      <button 
        onclick={() => activeTab = "download"}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm font-medium transition-all {activeTab === 'download' ? 'bg-blue-600 text-white shadow-md shadow-blue-600/20' : 'text-neutral-500 hover:bg-neutral-200/50 dark:hover:bg-neutral-800'}"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path></svg>
        影片下載
      </button>
      <button 
        onclick={() => activeTab = "transcode"}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm font-medium transition-all {activeTab === 'transcode' ? 'bg-blue-600 text-white shadow-md shadow-blue-600/20' : 'text-neutral-500 hover:bg-neutral-200/50 dark:hover:bg-neutral-800'}"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"></path></svg>
        影片轉檔
      </button>
      <button 
        onclick={() => { activeTab = "history"; loadHistory(); }}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm font-medium transition-all {activeTab === 'history' ? 'bg-blue-600 text-white shadow-md shadow-blue-600/20' : 'text-neutral-500 hover:bg-neutral-200/50 dark:hover:bg-neutral-800'}"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
        下載歷史
      </button>
      
      <div class="pt-4 mt-4 border-t border-neutral-200 dark:border-neutral-800">
        <a 
          href="/settings"
          class="w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm font-medium text-neutral-500 hover:bg-neutral-200/50 dark:hover:bg-neutral-800 transition-all"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
          軟體設定
        </a>
      </div>
    </nav>

    <div class="mt-auto pt-6 border-t border-neutral-200 dark:border-neutral-800">
      <div class="px-3 py-2 rounded-xl bg-neutral-200/30 dark:bg-neutral-800/30">
        <p class="text-[10px] uppercase font-bold text-neutral-400 mb-1">並行任務限制</p>
        <div class="flex items-center justify-between text-xs">
          <span>下載: 2</span>
          <span>轉檔: 1</span>
        </div>
      </div>
    </div>
  </aside>

  <!-- Main Content -->
  <main class="flex-1 overflow-y-auto p-10">
    {#if activeTab === "download"}
      <div in:fade={{ duration: 200 }}>
        <header class="mb-10">
          <h2 class="text-3xl font-extrabold tracking-tight mb-2">影片下載</h2>
          <p class="text-neutral-500 dark:text-neutral-400">貼上網址，我們會自動為您分類與下載</p>
        </header>

        {#if detectedUrl}
          <div 
            transition:slide
            class="mb-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-100 dark:border-blue-800 rounded-2xl flex items-center justify-between gap-4 shadow-sm"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div class="w-10 h-10 rounded-xl bg-blue-600 flex items-center justify-center flex-shrink-0 shadow-lg shadow-blue-500/20">
                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"></path></svg>
              </div>
              <div class="min-w-0">
                <p class="text-xs font-black text-blue-600 dark:text-blue-400 uppercase tracking-widest mb-0.5">偵測到剪貼簿網址</p>
                <p class="text-sm font-bold truncate dark:text-neutral-200">{detectedUrl}</p>
              </div>
            </div>
            <div class="flex items-center gap-2 flex-shrink-0">
              <button 
                onclick={() => { urlInput = detectedUrl; detectedUrl = ""; }}
                class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-xl text-xs font-black transition-all shadow-md shadow-blue-600/20 active:scale-95"
              >
                使用此連結
              </button>
              <button 
                onclick={() => detectedUrl = ""}
                class="p-2 text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200 transition-colors"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
              </button>
            </div>
          </div>
        {/if}

        <div class="flex gap-3 mb-10">
          <div class="flex-1 relative group">
            <input 
              bind:value={urlInput}
              onkeydown={(e) => e.key === 'Enter' && addDownloadTask()}
              placeholder="貼上 Facebook 或 Instagram 影片網址..." 
              class="w-full pl-5 pr-4 py-4 rounded-2xl border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 outline-none focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 transition-all shadow-sm group-hover:border-neutral-300 dark:group-hover:border-neutral-700"
            />
          </div>
          <button 
            onclick={addDownloadTask}
            disabled={!urlInput}
            class="px-8 py-4 rounded-2xl bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white font-bold shadow-lg shadow-blue-600/20 transition-all active:scale-95 flex items-center gap-2"
          >
            加入下載
          </button>
        </div>

        <div class="space-y-4">
          {#each downloadTasks as task (task.id)}
            <div 
              animate:flip={{ duration: 400 }}
              in:fly={{ y: 20, duration: 400 }}
              out:slide
              class="bg-white dark:bg-neutral-900 p-6 rounded-2xl border border-neutral-200 dark:border-neutral-800 shadow-sm flex flex-col gap-4 relative overflow-hidden"
            >
              {#if task.status === 'fetching' || task.status === 'downloading'}
                <div class="absolute top-0 left-0 h-1 bg-blue-600 transition-all duration-300" style="width: {task.progress}%"></div>
              {/if}

              <div class="flex justify-between items-start">
                <div class="flex-1 min-w-0 mr-6">
                  <div class="flex items-center gap-2 mb-1">
                    <span class="px-2 py-0.5 rounded-md text-[10px] font-black uppercase tracking-wider {task.source === 'Facebook' ? 'bg-blue-100 text-blue-700' : 'bg-pink-100 text-pink-700'}">
                      {task.source}
                    </span>
                    <span class="text-[10px] font-bold text-neutral-400 uppercase">{task.status}</span>
                  </div>
                  <h3 class="font-bold truncate text-lg">{task.title}</h3>
                </div>
                <div class="text-right">
                  {#if task.status === 'completed'}
                    <button onclick={() => downloadTasks = downloadTasks.filter(t => t.id !== task.id)} class="text-neutral-400 hover:text-red-500">
                      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
                    </button>
                  {:else if task.status === 'downloading'}
                    <span class="text-xl font-black text-blue-600">{task.progress}%</span>
                  {/if}
                </div>
              </div>

              {#if task.status === 'downloading'}
                <div class="flex items-center gap-4">
                  <div class="flex-1 h-2 bg-neutral-100 dark:bg-neutral-800 rounded-full overflow-hidden">
                    <div class="h-full bg-blue-600 transition-all duration-300" style="width: {task.progress}%"></div>
                  </div>
                  <span class="text-[10px] font-mono text-neutral-500">{task.speed}</span>
                </div>
              {/if}
              
              {#if task.status === 'completed'}
                <div class="flex justify-end gap-2">
                  <button onclick={() => openFile((task as any).file_path)} class="text-xs font-bold text-emerald-600 dark:text-emerald-400 hover:underline">開啟檔案夾</button>
                </div>
              {/if}
            </div>
          {:else}
            <div class="text-center py-20 bg-neutral-100/30 dark:bg-neutral-900/30 rounded-3xl border-2 border-dashed border-neutral-200 dark:border-neutral-800">
              <p class="text-neutral-400 font-medium">目前的佇列中沒有任務</p>
            </div>
          {/each}
        </div>
      </div>

    {:else if activeTab === "transcode"}
      <div in:fade={{ duration: 200 }}>
        <header class="mb-10">
          <h2 class="text-3xl font-extrabold tracking-tight mb-2">影片轉檔</h2>
          <p class="text-neutral-500 dark:text-neutral-400">強大的 ffmpeg 驅動，支援多種預設品質</p>
        </header>

        <div class="grid grid-cols-1 xl:grid-cols-4 gap-8">
          <div class="xl:col-span-3 space-y-6">
            <div 
              class="bg-white dark:bg-neutral-900 rounded-2xl p-8 shadow-sm border-2 border-dashed border-neutral-200 dark:border-neutral-800 text-center transition-all hover:border-blue-400 group"
              ondragover={handleDragOver}
            >
              <div class="w-16 h-16 bg-blue-50 dark:bg-blue-900/20 rounded-xl flex items-center justify-center mx-auto mb-4 group-hover:scale-110 transition-transform">
                <svg class="w-8 h-8 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
              </div>
              <h3 class="text-lg font-bold mb-1">選擇影片檔案</h3>
              <p class="text-neutral-400 mb-6 text-xs max-w-xs mx-auto">支援常見影片格式。拖放檔案至此處或點擊按鈕</p>
              <button 
                onclick={selectFiles}
                class="px-8 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-xl font-bold shadow-lg shadow-blue-600/20 transition-all active:scale-95 text-sm"
              >
                瀏覽檔案
              </button>
            </div>

            <div class="space-y-4">
              {#each transcodeTasks as task (task.id)}
                <div animate:flip in:fly={{ x: -20 }} class="bg-white dark:bg-neutral-900 p-6 rounded-2xl border border-neutral-200 dark:border-neutral-800 flex items-center gap-6 shadow-sm">
                  <div class="w-12 h-12 bg-neutral-100 dark:bg-neutral-800 rounded-xl flex items-center justify-center flex-shrink-0">
                    <svg class="w-6 h-6 text-neutral-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                  </div>
                  <div class="flex-1 min-w-0">
                    <h4 class="font-bold truncate text-base">{task.fileName}</h4>
                    <p class="text-[10px] text-neutral-400 truncate">{task.inputPath}</p>
                    {#if task.status === 'processing'}
                      <div class="mt-2 flex items-center gap-4">
                        <div class="flex-1 h-1.5 bg-neutral-100 dark:bg-neutral-800 rounded-full overflow-hidden">
                          <div class="h-full bg-blue-600" style="width: {task.progress}%"></div>
                        </div>
                        <span class="text-[10px] font-mono font-bold text-blue-600">{task.progress.toFixed(1)}%</span>
                      </div>
                    {/if}
                  </div>
                  <div class="flex-shrink-0 flex items-center gap-3">
                    {#if task.status === 'pending'}
                      <button onclick={() => startTranscode(task)} class="p-3 bg-blue-50 text-blue-600 rounded-xl hover:bg-blue-600 hover:text-white transition-all shadow-sm">
                        <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20"><path d="M4.5 3.5v13L16 10l-11.5-6.5z"/></svg>
                      </button>
                    {:else if task.status === 'completed'}
                      <button 
                        onclick={() => openFile(task.outputPath || "")}
                        class="px-4 py-2 bg-emerald-50 text-emerald-600 rounded-lg text-xs font-bold hover:bg-emerald-600 hover:text-white transition-all"
                      >
                        開啟檔案
                      </button>
                      <div class="w-8 h-8 rounded-full bg-emerald-100 dark:bg-emerald-900/30 flex items-center justify-center text-emerald-600">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"></path></svg>
                      </div>
                    {:else if task.status === 'failed'}
                      <span class="text-red-500 font-bold text-xs">失敗</span>
                    {/if}
                    <button onclick={() => transcodeTasks = transcodeTasks.filter(t => t.id !== task.id)} class="text-neutral-400 hover:text-red-500 p-1">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
                    </button>
                  </div>
                </div>
              {:else}
                <div class="text-center py-12 bg-neutral-100/50 dark:bg-neutral-900/50 rounded-2xl border-2 border-dashed border-neutral-200 dark:border-neutral-800 text-neutral-400">
                  尚無轉檔任務
                </div>
              {/each}
            </div>
          </div>

          <!-- Settings Sidebar -->
          <div class="space-y-6">
            <div class="bg-white dark:bg-neutral-900 rounded-3xl p-6 border border-neutral-200 dark:border-neutral-800 shadow-sm sticky top-10">
              <h3 class="font-black text-xs uppercase tracking-widest text-neutral-400 mb-6 flex items-center gap-2">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"></path></svg>
                轉檔配置
              </h3>
              
              <div class="space-y-6">
                <div class="grid grid-cols-1 gap-2">
                  {#each ["high", "balanced", "fast", "custom"] as p}
                    <button 
                      onclick={() => setPreset(p)}
                      class="px-4 py-3 rounded-xl text-xs font-bold transition-all border {globalOptions.preset === p ? 'bg-blue-600 border-blue-600 text-white shadow-lg shadow-blue-600/20' : 'bg-neutral-50 dark:bg-neutral-800 border-neutral-100 dark:border-neutral-700 text-neutral-500 hover:border-neutral-300'}"
                    >
                      {p === 'high' ? '高品質' : p === 'balanced' ? '平衡' : p === 'fast' ? '快速 (720p)' : '自定義'}
                    </button>
                  {/each}
                </div>

                <div class="pt-6 border-t border-neutral-100 dark:border-neutral-800 space-y-4">
                  <div class="flex justify-between items-center px-1">
                    <span class="text-[10px] font-bold text-neutral-400 uppercase">進階設定</span>
                    <button onclick={() => showAdvanced = !showAdvanced} class="text-blue-600 font-bold text-xs">{(showAdvanced || globalOptions.preset === "custom") ? '隱藏' : '展開'}</button>
                  </div>

                  {#if showAdvanced || globalOptions.preset === "custom"}
                    <div class="space-y-4" transition:slide>
                      <div class="space-y-1.5">
                        <label class="text-[10px] font-bold text-neutral-500 ml-1">影片編碼</label>
                        <select bind:value={globalOptions.codec} class="w-full bg-neutral-100 dark:bg-neutral-800 border-none rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-blue-500 transition-all outline-none">
                          <option value="h264">H.264 (AVC)</option>
                          <option value="h265">H.265 (HEVC)</option>
                        </select>
                      </div>
                      <div class="space-y-1.5">
                        <label class="text-[10px] font-bold text-neutral-500 ml-1">解析度</label>
                        <select bind:value={globalOptions.resolution} class="w-full bg-neutral-100 dark:bg-neutral-800 border-none rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-blue-500 transition-all outline-none">
                          <option value="original">原始大小</option>
                          <option value="1080">1080p</option>
                          <option value="720">720p</option>
                          <option value="480">480p</option>
                        </select>
                      </div>
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

    {:else if activeTab === "history"}
      <div in:fade={{ duration: 200 }}>
        <header class="mb-10 flex justify-between items-end">
          <div>
            <h2 class="text-3xl font-extrabold tracking-tight mb-2">下載歷史</h2>
            <p class="text-neutral-500 dark:text-neutral-400">管理您過去下載的珍貴影片</p>
          </div>
          <button onclick={loadHistory} class="p-3 bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl hover:bg-neutral-100 transition-colors shadow-sm">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.001 0 01-15.357-2m15.357 2H15"></path></svg>
          </button>
        </header>

        <div class="space-y-2">
          {#each history as item (item.id)}
            <div class="bg-white dark:bg-neutral-900 px-6 py-4 rounded-2xl border border-neutral-200 dark:border-neutral-800 flex items-center gap-6 hover:border-neutral-300 dark:hover:border-neutral-700 transition-all group">
              <!-- Source Icon -->
              <div class="w-10 h-10 rounded-xl bg-neutral-100 dark:bg-neutral-800 flex items-center justify-center flex-shrink-0">
                <span class="font-black text-lg {item.source === 'Facebook' ? 'text-blue-600' : 'text-pink-600'}">
                  {item.source ? item.source[0].toLowerCase() : '?'}
                </span>
              </div>
              
              <!-- Info -->
              <div class="flex-1 min-w-0">
                <h3 class="font-bold truncate text-neutral-800 dark:text-neutral-200 mb-0.5">{item.title || '未命名影片'}</h3>
                <div class="flex items-center gap-3">
                  <span class="text-[10px] font-bold text-neutral-400 uppercase tracking-widest">{new Date(item.created_at).toLocaleDateString()}</span>
                  <div class="w-1 h-1 bg-neutral-300 dark:bg-neutral-700 rounded-full"></div>
                  <span class="text-[10px] font-black {item.status === 'completed' ? 'text-emerald-500' : 'text-amber-500'} uppercase">{item.status}</span>
                </div>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-2">
                {#if item.status === 'completed'}
                  <button 
                    onclick={() => openFile(item.file_path)} 
                    class="p-2.5 bg-emerald-50 dark:bg-emerald-900/20 text-emerald-600 dark:text-emerald-400 rounded-xl hover:bg-emerald-600 hover:text-white transition-all shadow-sm"
                    title="開啟檔案夾"
                  >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19l7-7 7 7M5 5l7 7 7-7"></path></svg>
                  </button>
                {/if}
                
                <button 
                  onclick={() => retryFromHistory(item)} 
                  class="p-2.5 bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 rounded-xl hover:bg-blue-600 hover:text-white transition-all shadow-sm"
                  title="重新下載"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.001 0 01-15.357-2m15.357 2H15"></path></svg>
                </button>

                <button 
                  onclick={() => deleteHistoryRecord(item.id)} 
                  class="p-2.5 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-xl hover:bg-red-600 hover:text-white transition-all shadow-sm"
                  title="刪除紀錄"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                </button>
              </div>
            </div>
          {:else}
            <div class="text-center py-20 bg-neutral-100/30 dark:bg-neutral-900/30 rounded-3xl border-2 border-dashed border-neutral-200 dark:border-neutral-800">
              <p class="text-neutral-400 font-medium">目前沒有下載紀錄</p>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    user-select: none;
    background-color: transparent;
  }
</style>
