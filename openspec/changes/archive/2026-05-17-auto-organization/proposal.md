## Why

使用者希望下載的影片能依據來源（Facebook/Instagram）自動分類儲存，以便於檔案管理。

## What Changes

- 更新 `download_video` 指令，支援自動建立來源資料夾。
- 在下載時自動將檔案存入 `Downloads/VidBridge/{Source}/`。

## Capabilities

### New Capabilities

- `download-auto-organization`: 依據來源自動建立目錄並儲存影片。

### Modified Capabilities

(none)

## Impact

- Affected specs: `download-auto-organization`
- Affected code:
  - Modified: `src-tauri/src/commands/download.rs`, `src/routes/+page.svelte`
