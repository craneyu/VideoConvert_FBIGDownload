## Why

下載後處理目前在兩個地方無謂地損失畫質與音質，而其中一項完全沒有取捨可言。

**音訊被無條件重新編碼。** 判定為重新編碼時，音訊一律轉成 AAC 128k，即使來源本來就是 AAC。以一支真實的 Facebook Reel 實測：來源音訊是 AAC 59959 bps，輸出被拉到 128385 bps —— **檔案變大、音質經過二次壓縮而變差、還多花時間**。而判斷所需的資訊早就有了：ffprobe 探測結果裡的音訊 codec 欄位目前只被用來決定白名單，沒有被用來決定音訊要不要重編。

**視訊被重新編碼，即使原檔可以直接使用。** 同一支 Reel 的實測數據（18.5 秒）：

| 方案 | 大小 | 後處理耗時 | 解析度 | SSIM（對照下載到的原檔） |
|---|---|---|---|---|
| 下載到的 AV1 原檔 | 6.50 MB | — | 1080x1920 | 基準 |
| 目前：重新編碼為 H.264 | 9.93 MB | 6 秒 | 1080x1920 | 0.9847 |
| 直接 remux 原檔 | 6.50 MB | 0.03 秒 | 1080x1920 | 無損（位元相同） |

目前路徑在畫質、檔案大小、耗時三項**全部**輸給直接 remux —— 檔案大 53%、慢 200 倍、還掉畫質。重新編碼的唯一理由是**播放相容性**：Facebook 的 1080p 只供 AV1，而 AV1 在較舊的 Intel Mac 與未安裝 AV1 擴充的 Windows 10 上無法播放。

**為什麼是現在**：`Post-Download Container Optimization` 的白名單只認 H.264，所以每一支 Reel 都走完整重新編碼。而編碼是共用 CPU 預算裡最昂貴的操作 —— 一支 27 分鐘的影片曾實測花掉 40 分鐘以上。放寬到可以直接使用原檔，同時解決畫質與速度。

## What Changes

分為三組。C 組含一項對既有使用者可見的行為變更。

### A. 音訊在來源已是 AAC 時直接複製

- 判定為重新編碼時，若探測到的音訊 codec 已是 AAC，音訊改為直接複製而不重新編碼。非 AAC（或無法探測）才轉成 AAC。
- 此項無取捨：檔案更小、音質不再二次壓縮、耗時略減。不需要任何設定。

### B. 新增平台編解碼能力偵測（新 capability）

- 新增一個模組回答「本平台能否解碼某個視訊 codec」，回傳三態：**可以**、**不行**、**未知**。
- macOS 透過 VideoToolbox 的硬體解碼查詢。Windows 與 Linux 在本變更中一律回傳未知：Linux 沒有能代表使用者播放器能力的系統答案；Windows 的 Media Foundation 查詢是 COM API，需要額外依賴且無法在 macOS 上編譯或測試（此專案的 Windows target 在 macOS 上會於 ring 的 C 編譯階段失敗），因此獨立成另一個能在 Windows 或 CI 上真正驗證的變更。
- 只有明確回答「可以」才被視為支援。「未知」與「不行」在後續決策中一律走保守路徑。macOS 的查詢只涵蓋硬體解碼，因此具備軟體解碼能力的機器會被低估為不支援 —— 這是刻意選擇的保守方向，且使用者可手動覆寫。

### C. 新增下載視訊處理策略設定

- 新增一個設定項，三個值：
  - `auto`（預設）：依 B 組的偵測結果決定 —— 平台能解碼原始視訊 codec 就直接 remux，否則重新編碼。
  - `original`：一律嘗試直接 remux，保留原始畫質。
  - `compat`：一律重新編碼為 H.264，維持目前行為。
- **BREAKING（行為變更）**：在偵測到支援 AV1 的機器上，預設行為由「重新編碼為 H.264」變為「直接 remux」。輸出檔案會是 AV1 而非 H.264 —— 畫質更好、檔案更小、幾乎瞬間完成，但把該檔案帶到不支援 AV1 的裝置上會無法播放。設定介面須說明這個取捨，並提供切換到 `compat` 的途徑。
- 設定介面須顯示 `auto` 目前解析到的結果（例如「已偵測到本機支援 AV1，將保留原始畫質」），否則使用者無法得知 `auto` 實際做了什麼。

## Capabilities

### New Capabilities

- `platform-codec-capability`: 回答「本平台能否解碼指定視訊 codec」的三態偵測 —— macOS 走 VideoToolbox，其餘平台回傳未知；只有明確的「可以」才算支援，其餘一律保守。

### Modified Capabilities

- `video-download-engine`: 後處理決策由固定白名單改為「白名單 + 處理策略 + 平台能力」三者共同決定；音訊在來源已是 AAC 時直接複製
- `settings-management`: 新增下載視訊處理策略設定鍵及其預設值

## Impact

- Affected specs: `platform-codec-capability`、`video-download-engine`、`settings-management`
- Affected code:
  - New: `src-tauri/src/commands/codec_support.rs`
  - Modified: `src-tauri/src/commands/download.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/commands/settings.rs`、`src-tauri/src/lib.rs`、`src/lib/stores/settings.svelte.ts`、`src/routes/settings/+page.svelte`、`src-tauri/Cargo.toml`
  - Removed: (none)
- Dependencies: macOS 需連結 VideoToolbox 系統框架（C 函式，以 extern 宣告使用，不引入第三方套件）。Windows 的 Media Foundation 查詢不在本變更範圍內。無資料庫 migration —— 設定沿用既有 key-value 表與 Settings 的合併邏輯。
- 與 `concurrency-budget` 的互動：放寬後更多任務走 remux，而 remux 依既有規格不佔用 CPU 預算，因此共用編碼名額的競爭會自然下降。該 capability 的規格不需修改。
