## Why

使用者需要更細緻的轉檔控制，例如選擇特定品質預設值或自定義解析度與編碼，以平衡轉檔速度、檔案大小與影片品質。

## What Changes

- 更新 Rust 轉檔指令，支援自定義參數（Preset, Resolution, Codec）。
- 在前端實作轉檔設定面板，包含預設組合與進階設定切換。
- 整合 ffmpeg 參數動態建構邏輯。

## Capabilities

### New Capabilities

- `transcoding-config-management`: 提供多樣化的轉檔配置選項。

### Modified Capabilities

- `video-transcoding-engine`: 擴展轉檔引擎以支援自定義選項參數。

## Impact

- Affected specs: `video-transcoding-engine`, `transcoding-config-management`
- Affected code:
  - Modified: `src-tauri/src/commands/transcode.rs`, `src/routes/+page.svelte`
