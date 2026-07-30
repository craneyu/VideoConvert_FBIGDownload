## Why

VidBridge 的 README 宣稱「支援多執行緒下載與影片格式轉換（開發中）」，但兩者的實況與這句話都不符：影片格式轉換早已完整出貨，多任務並行下載也在跑（固定 2）。真正沒做的是**讓並行度變成有意義的東西** —— 而目前的架構會讓「提高並行度」把程式變慢。

三個互相咬合的成因：

**下載與轉檔各自為政地搶同一份 CPU。** 下載命令做完 yt-dlp 之後，會接著跑一段 ffmpeg 後處理；轉檔頁籤則是另一條完全獨立的 ffmpeg 管線。兩邊都不知道對方在做什麼。而 libx264 本來就會吃滿所有核心，因此兩個並行的重新編碼是把每個砍半，不是加總吞吐 —— 上調下載並行度的直接後果是總時間變長。

**轉檔完全沒有上限。** 側邊欄顯示「轉檔: 1」，但那是硬寫死的文字，背後沒有任何佇列或計數：轉檔頁籤的每個任務各有一顆手動啟動按鈕，連點五次就同時開五個 ffmpeg 行程。

**下載與轉檔命令都是 async 卻做同步阻塞 I/O。** 兩者都在命令內直接逐行讀取子行程輸出並等待其結束，因此整趟工作期間佔住一個 async runtime 的 worker thread。這使得任何上調並行度的動作都會擠壓 runtime，讓設定讀寫、歷史查詢等其他 IPC 一起卡住。

**為什麼是現在**：並行上限目前是前端的一個常數，而側邊欄另外硬寫了一份顯示值，兩處已經必須手動保持一致；設定頁沒有對應欄位。在補上第三處之前先把預算收攏成單一來源，成本最低。

## What Changes

分為三組，彼此互為前置。

### A. 共用執行預算（新 capability）

- 新增一個 Rust 端的執行預算模組，持有**單一個 CPU 許可池**。網路並行度不在 Rust 端複製一份計數 —— 前端本來就控制同時發出幾個下載命令，那就是網路並行度。
- CPU 許可池由「下載後處理」與「轉檔」**共用**，因此兩條管線加起來的重新編碼數量受同一個上限約束。這是整個設計的核心 —— 網路平行度便宜、CPU 平行度昂貴，兩者綁在同一個命令裡就無法分別計價。
- 網路名額與 CPU 名額**分階段**計價：下載完成其 yt-dlp 階段後即釋放網路名額，不因等待 CPU 許可而繼續佔住它。這需要下載在進入等待前先回報狀態，讓前端佇列據以推進。
- **僅重新編碼需要 CPU 許可**。容器 remux 是串流複製（不解碼、不編碼），成本接近純 I/O，取得 CPU 許可只會讓它無謂排隊。下載流程已有現成的決策結果可據以判斷該階段屬於哪一種。
- 下載與轉檔命令改為不佔用 async runtime 的 worker thread。

### B. 並行度可設定

- 網路並行度與 CPU 並行度成為兩個設定項，取代前端的固定常數。
- 預設值：網路 3、CPU 1。CPU 上限只開放到 2，且介面須說明那是「讓程式保持回應」而非「更快」，因為第二個並行編碼是把每個砍半而非加總吞吐。
- 側邊欄的並行任務限制顯示改為讀取實際設定值，不再是硬寫死的文字。

### C. 佇列行為補齊

- 轉檔任務改為受佇列約束：超過 CPU 上限的任務進入等待，而非立刻開新的 ffmpeg 行程。
- 新增「等待編碼」任務狀態。這是分階段許可的直接後果：多支影片可能都已下載完成卻排在 CPU 許可後面，進度條全部停在下載階段的上限值。沒有這個狀態，那些任務看起來就是當機。
- README 中「支援多執行緒下載與影片格式轉換（開發中）」改寫為實際成立的描述。

## Capabilities

### New Capabilities

- `concurrency-budget`: 跨下載後處理與轉檔兩條管線的共用 CPU 許可池 —— 網路名額與 CPU 名額分階段計價、僅重新編碼計入 CPU 預算、取得許可失敗不得靜默略過，以及長時間執行的命令不得阻塞 async runtime 的要求。

### Modified Capabilities

- `download-queue-management`: 並行上限由規格寫死的 2 改為可設定；新增「等待編碼」佇列狀態
- `transcoding-ui-integration`: 轉檔任務須受並行上限約束（目前規格完全沒有這條要求，實作也沒有）
- `settings-management`: 新增網路並行度與 CPU 並行度兩個設定項

## Impact

- Affected specs: `concurrency-budget`、`download-queue-management`、`transcoding-ui-integration`、`settings-management`
- Affected code:
  - New: `src-tauri/src/commands/concurrency.rs`
  - Modified: `src-tauri/src/commands/download.rs`、`src-tauri/src/commands/transcode.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/commands/settings.rs`、`src-tauri/src/lib.rs`、`src/routes/+page.svelte`、`src/routes/settings/+page.svelte`、`src/lib/stores/settings.svelte.ts`、`README.md`
  - Removed: (none)
- Dependencies: 無新增外部依賴。設定以既有的 key-value 表儲存並經由 Settings 的合併邏輯讀出，因此不需要新的資料庫 migration。
