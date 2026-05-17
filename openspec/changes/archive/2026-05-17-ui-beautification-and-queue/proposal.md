## Why

目前應用程式介面較為陽春，且一次僅能執行單一網址下載，這限制了使用者的效率。美化介面與引入任務佇列管理將顯著提升操作體驗。

## What Changes

- 前端介面全面重構，採用更具現代感的 macOS 風格（Sidebar, Card based UI）。
- 實作下載任務佇列，支援批次加入網址並限制同時下載數量（並行上限 2）。
- 加入 Svelte 內建動畫與過渡效果，提升互動流暢度。

## Capabilities

### New Capabilities

- `download-queue-management`: 管理多個下載任務的狀態與執行順序。
- `enhanced-ui-experience`: 提供現代化、具備動畫效果的介面。

### Modified Capabilities

(none)

## Impact

- Affected specs: `download-queue-management`, `enhanced-ui-experience`
- Affected code:
  - Modified: `src/routes/+page.svelte`
