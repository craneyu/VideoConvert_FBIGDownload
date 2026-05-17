## Context

目前 VidBridge 的下載與轉檔設定散落在程式碼各處，或是僅存於記憶體中。為了提供更好的使用者體驗，需要一個持久化的設定系統來儲存使用者偏好，如預設下載路徑、轉檔品質預設值等。

## Goals / Non-Goals

**Goals:**
- 實作基於 SQLite 的鍵值對（Key-Value）設定儲存機制。
- 提供強型別的後端設定模型與前端同步機制。
- 整合現有的轉檔與下載模組，使其優先使用全域設定。

**Non-Goals:**
- 本次不實作多使用者設定檔（Profiles）。
- 不包含遠端同步功能（Cloud Sync）。

## Decisions

### 使用單一資料表儲存設定
決定在 SQLite 中建立 `settings` 資料表。欄位包含 `key` (TEXT, PRIMARY KEY) 與 `value` (TEXT)。
- **理由**：Key-Value 形式最適合儲存雜亂的配置項，且易於擴充。
- **備選方案**：使用 JSON 檔案。但由於專案已整合 `tauri-plugin-sql`，直接使用資料庫更具一致性且能保證交易安全性。

### 後端預設值合併機制
在 Rust 後端定義 `Settings` 結構體並實作 `Default` 特性。讀取時，資料庫的內容將與預設值進行合併（Merge）。
- **理由**：確保當資料庫尚未初始化或新增設定項時，前端仍能獲得有效的數值。

### Svelte 5 全域 Store 同步
使用 Svelte 5 的 `` 建立全域 Store。在 App 啟動時呼叫 `get_settings` 初始化 Store，每次更新時透過 `update_setting` 同步至後端。
- **理由**：簡化組件間的狀態共享，並確保 UI 能即時反應設定變更。

## Implementation Contract

- **後端介面**：
  - `get_settings() -> Settings`：回傳合併後的完整設定物件。
  - `update_setting(key: String, value: serde_json::Value) -> Result`：更新特定設定項。
- **資料模型**：
  - `download_path`: 字串，預設為系統下載資料夾。
  - `auto_organize`: 布林值，預設為 false。
  - `transcoding_preset`: 字串，可選值為 'High', 'Balanced', 'Fast'。
- **驗證標準**：
  - 修改設定後重啟 App，設定值必須維持不變。
  - 轉檔功能在未手動調整參數時，應套用設定中的預設 Preset。

## Risks / Trade-offs

- [Risk] 資料庫遷移失敗 → [Mitigation] 在 `lib.rs` 中使用強韌的遷移指令，並在失敗時記錄日誌。
- [Risk] 下載路徑權限問題 → [Mitigation] 在更新路徑時進行基本的資料夾存取權限檢查。