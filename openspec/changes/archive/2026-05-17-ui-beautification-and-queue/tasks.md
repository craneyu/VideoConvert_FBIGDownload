## 1. 佇列邏輯實作 (Queue Implementation)

- [x] 1.1 定義 `DownloadTask` 數據結構並重構前端狀態管理，實作 Concurrent Download Limit (並行限制) 邏輯，驗證方式：加入多個連結並確認僅有兩個在執行。
- [x] 1.2 實作 `processQueue` 自動調度器，確保 Automatic Queue Progression (自動遞補)，驗證方式：完成一個下載後下一個待辦任務自動啟動。

## 2. 介面美化與重構 (UI Beautification)

- [x] 2.1 重構 `+page.svelte` 佈局，實作 Modern macOS Aesthetic (現代化美學) 設計，包含 Sidebar 與 Card 佈局，驗證方式：視覺檢查 UI 是否符合設計預期。
- [x] 2.2 加入 Svelte `slide` 與 `fade` 過渡效果，實現 Animated Task Transitions (動畫過渡)，驗證方式：觀察任務加入與狀態切換時的動畫。
- [x] 2.3 優化轉檔設定面板與歷史列表的視覺表現，驗證方式：確認不同 Tab 之間的視覺一致性。