## Context

VidBridge 已經建立了基礎開發環境。現在需要實作核心功能之一：影片下載。我們決定使用 `yt-dlp` 作為下載引擎，並整合 `tauri-plugin-sql` 來管理下載歷史。

## Goals / Non-Goals

**Goals:**
- 整合 SQLite 插件並實作資料庫遷移 (Migration)。
- 實作 Rust 層的 `yt-dlp` 執行邏輯與進度解析。
- 在前端實作下載任務管理與歷史紀錄顯示。

**Non-Goals:**
- 實作下載續傳 (Resume) 功能。
- 實作下載任務的佇列 (Queue) 排序管理。

## Decisions

- **使用 tauri-plugin-sql**: 這是 Tauri 官方支援的 SQL 插件，支援 SQLite，且允許在前端直接進行簡單的查詢，降低開發複雜度。
- **解析 yt-dlp 輸出**: 透過 `std::process::Command` 執行 `yt-dlp`，並解析其標準輸出 (stdout) 中的進度百分比，轉換為前端可用的 Event。
- **資料夾命名規則**: 下載影片將依據來源（FB/IG）自動存入 `Downloads/VidBridge/{Source}/` 資料夾。

## Implementation Contract

- **行為 (Behavior)**: 使用者輸入網址後，系統先獲取資訊並顯示，點擊下載後啟動後台進程。下載完成後通知使用者並更新歷史紀錄。
- **介面與數據 (Interface)**: 
  - 資料表 `download_history`: `id`, `url`, `title`, `status`, `file_path`, `source`, `created_at`。
  - Tauri Events: `download-progress` (payload: { id, progress, speed })。
- **驗證標準 (Acceptance Criteria)**:
  - 能夠下載公開的 FB/IG 影片。
  - 下載進度條在 UI 上能即時更新。
  - 重新啟動 App 後，歷史紀錄仍能正確顯示。

## Risks / Trade-offs

- [風險] yt-dlp 輸出格式變更 → [對策] 使用強健的正則表達式解析進度，若解析失敗則僅顯示「下載中」。
- [風險] 資料庫寫入衝突 → [對策] 使用簡單的 SQLite 交易處理。