## Why

為了確保使用者始終能夠使用最新版本的 VidBridge，特別是針對 yt-dlp 等下載引擎的頻繁更新，建立自動更新機制是必要的。

## What Changes

- 整合 `tauri-plugin-updater` 插件。
- 配置 `tauri.conf.json` 以支援從 GitHub Releases 獲取更新。
- 在前端實作更新檢查邏輯，並在發現新版本時提示使用者。

## Capabilities

### New Capabilities

- `app-auto-update`: 偵測、下載並安裝應用程式更新。

### Modified Capabilities

(none)

## Impact

- Affected specs: `app-auto-update`
- Affected code:
  - Modified: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src/routes/+page.svelte`
