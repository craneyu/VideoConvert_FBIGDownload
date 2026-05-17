## Why

使用者需要下載 Facebook 與 Instagram 的影片，且希望能夠在應用程式中查看過去的下載紀錄以便管理。

## What Changes

- 整合 `tauri-plugin-sql` 以支援 SQLite 資料庫。
- 實作基於 `yt-dlp` 的影片下載引擎。
- 新增下載歷史介面，顯示下載進度與過去的紀錄。

## Capabilities

### New Capabilities

- `video-download-engine`: 執行 yt-dlp 下載影片並回傳進度。
- `download-history-storage`: 使用 SQLite 儲存與查詢下載紀錄。
- `download-ui-integration`: 提供下載輸入與歷史列表介面。

### Modified Capabilities

(none)

## Impact

- Affected specs: `video-download-engine`, `download-history-storage`, `download-ui-integration`
- Affected code:
  - New: `src-tauri/src/commands/download.rs`
  - Modified: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `package.json`, `src/routes/+page.svelte`
