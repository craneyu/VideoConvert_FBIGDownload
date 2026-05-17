## Why

使用者需要一個簡單且高效的方式來轉換影片格式，以便在不同裝置上播放或縮減檔案大小。

## What Changes

- 實作基於 `ffmpeg` 的影片轉檔引擎。
- 提供影片品質設定（解析度、比特率等）。
- 支援批次轉檔與拖放上傳。
- 顯示即時轉檔進度。

## Capabilities

### New Capabilities

- `video-transcoding-engine`: 執行 ffmpeg 進行轉檔並解析進度。
- `transcoding-ui-integration`: 提供影片選擇、格式設定與進度顯示介面。

### Modified Capabilities

(none)

## Impact

- Affected specs: `video-transcoding-engine`, `transcoding-ui-integration`
- Affected code:
  - New: `src-tauri/src/commands/transcode.rs`
  - Modified: `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src/routes/+page.svelte` (或新增 Tab)
