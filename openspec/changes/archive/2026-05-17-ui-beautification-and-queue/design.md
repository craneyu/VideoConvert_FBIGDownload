## Context

此變更專注於提升 VidBridge 的視覺質感與下載效率。我們將從單一任務模式轉換為任務佇列模式。

## Goals / Non-Goals

**Goals:**
- 全面重設計 `+page.svelte` 的 UI 結構。
- 實作前端 `processQueue` 邏輯以控制並發下載。
- 使用 Svelte \`crossfade\` 或 \`animate:flip\` 處理列表變動。

**Non-Goals:**
- 實作持久化的未完成任務佇列（目前僅記憶體中管理，重啟後清空，但 SQLite 歷史不受影響）。

## Implementation Contract

- **行為 (Behavior)**: 使用者點擊「下載」後網址框立即清空，任務進入列表。佇列處理器會自動挑選任務執行。
- **數據 (Data)**: 前端狀態 \`downloadTasks\` 陣列包含所有活動與待辦任務。
- **驗證標準 (Acceptance Criteria)**: 加入 3 個任務，觀察進度條是否只有前兩個在跳動。

## Risks / Trade-offs

- [風險] 記憶體佔用過高 → [對策] 限制同時轉檔與下載的數量。