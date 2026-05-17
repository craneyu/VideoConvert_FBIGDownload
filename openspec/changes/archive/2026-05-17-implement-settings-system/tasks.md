## 1. 資料庫與後端基礎

- [x] 1.1 實作資料庫遷移以建立 `settings` 資料表。驗證方式：啟動 App 後檢查 `vidbridge.db` 是否包含該資料表，滿足 Persistent Settings Storage 需求。
- [x] 1.2 在 Rust 後端實作 `Settings` 結構體與 `Default` 特性。驗證方式：撰寫單元測試驗證 `Settings::default()` 的數值是否符合 Default Settings Values 需求。
- [x] 1.3 實作後端預設值合併機制，確保資料庫內容能與 Default 結構體合併。驗證方式：當資料庫內容不完全時，讀取結果仍包含所有必要欄位，落實「後端預設值合併機制」決策。

## 2. IPC 指令實作

- [x] 2.1 [P] 實作 `get_settings` 指令以回傳合併後的設定物件。驗證方式：透過 Tauri Invoke 呼叫並確認回傳 JSON 格式正確，滿足 Global Settings Access 需求。
- [x] 2.2 [P] 實作 `update_setting` 指令以更新特定的鍵值對。驗證方式：更新後再次呼叫 `get_settings` 確認數值已變更，滿足 Updating Settings 需求。
- [x] 2.3 在 `src-tauri/src/commands/mod.rs` 註冊新的設定相關指令，並將實作放在新的 `src-tauri/src/commands/settings.rs` 中。驗證方式：前端可成功 Invoke 這些指令，實現「使用單一資料表儲存設定」決策。

## 3. 前端 Store 與 UI

- [x] 3.1 [P] 使用 Svelte 5 的 `$state` 建立全域 Store 並在初始化時讀取後端設定。驗證方式：檢查 Store 初始值是否與後端同步，落實「Svelte 5 全域 Store 同步」決策。
- [x] 3.2 [P] 建立 `src/routes/settings/+page.svelte` 頁面與基本的 UI 元件。驗證方式：使用者可導覽至 `/settings` 並看到配置項。
- [x] 3.3 實作下載路徑選擇器與其他設定控制項（開關、下拉選單）。驗證方式：手動操作 UI 後，前端 Store 與資料庫均即時同步更新。

## 4. 模組整合

- [x] 4.1 修改轉檔模組，使其預設採用設定系統中的 Transcoding Quality Presets 與 Advanced Resolution Control。驗證方式：啟動轉檔時，預設帶入的參數應與設定頁面一致。
- [x] 4.2 修改下載模組，使其下載路徑與 Source-Based Directory Creation 行為受全域設定控制。驗證方式：當 `auto_organize` 為 false 時，檔案應直存於下載路徑，滿足 Categorized File Saving 需求。