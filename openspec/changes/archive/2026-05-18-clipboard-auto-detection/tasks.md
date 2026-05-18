## 1. 基礎架構與設定整合 (Infrastructure & Settings Integration)

- [x] 1.1 [P] 在 src-tauri/src/commands/utils.rs 或相關位置實作 read_clipboard_text 指令，並在 src-tauri/src/lib.rs 註冊，透過手動調用 invoke 驗證可讀取剪貼簿。
- [x] 1.2 [P] 更新 src/lib/stores/settings.svelte.ts 與後端資料庫架構，實作 Clipboard Detection Toggle (detect_clipboard)，並透過設定頁面手動開關驗證 狀態管理整合。

## 2. 核心偵測邏輯實作 (Core Detection Logic)

- [x] 2.1 [P] 在 src/routes/+page.svelte 中實作 採用事件驅動的偵測機制，透過 window focus 事件觸發網址檢查，並利用 console.log 驗證觸發頻率。
- [x] 2.2 [P] 撰寫並整合 嚴格的網址 Regex 過濾 邏輯，實作 Clipboard Content Identification，過濾出有效 FB/IG 影片網址，並透過不同輸入範例驗證正確性。

## 3. 使用者介面與交互 (UI & Interaction)

- [x] 3.1 [P] 實作 Detected URL Prompt 的 UI 提示條元件，當偵測到有效網址時顯示於下載頁面，並手動驗證其顯示與消失邏輯。
- [x] 3.2 整合自動填入功能，當使用者點選提示條時，自動將偵測到的網址填入輸入框，完成 Active Window Focus Detection 的完整閉環，並手動測試端到端流程。
