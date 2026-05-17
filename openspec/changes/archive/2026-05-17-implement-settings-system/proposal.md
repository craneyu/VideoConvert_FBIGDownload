## Why

目前 VidBridge 缺乏集中式的設定管理系統，導致轉檔預設值、下載路徑以及自動分類等功能無法持久化保存。實作設定系統能提升使用者體驗，讓軟體能夠記住使用者的偏好設定，並在不同模組間共享配置。

## What Changes

- 資料庫擴充：在 SQLite 中新增 settings 資料表，用於存取鍵值對（key-value）形式的設定。
- 後端指令：新增用於讀取、更新及重置設定的 Tauri 指令（IPC commands）。
- 前端介面：新增「設定」頁面或彈窗，讓使用者能直觀地調整下載路徑、轉檔品質等選項。
- 模組整合：將現有的下載與轉檔功能連接至設定系統，自動套用使用者定義的預設配置。

## Capabilities

### New Capabilities

- settings-management: 提供全域設定的持久化儲存、讀取與管理介面，包含下載路徑、預設轉檔參數等。

### Modified Capabilities

- transcoding-config-management: 將原本硬編碼或暫存的轉檔參數改為從設定系統讀取。
- download-auto-organization: 將自動分類的開關與規則整合至全域設定中。

## Impact

- Affected specs: settings-management (new), transcoding-config-management, download-auto-organization
- Affected code:
  - New: src-tauri/src/commands/settings.rs, src/routes/settings/+page.svelte, src/lib/stores/settings.ts
  - Modified: src-tauri/src/lib.rs, src-tauri/src/commands/mod.rs, src/routes/+layout.svelte