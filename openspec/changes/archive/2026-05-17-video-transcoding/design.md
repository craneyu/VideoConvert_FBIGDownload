## Context

使用者需要一個高效的影片轉檔工具。我們已經決定依賴系統安裝的 `ffmpeg`。此設計將專注於如何在 Rust 中執行並解析 `ffmpeg` 的輸出，以及如何在前端管理多個並行的轉檔任務。

## Goals / Non-Goals

**Goals:**
- 實作 Rust 層的 `ffmpeg` 執行邏輯。
- 解析 `ffmpeg` 的標準錯誤輸出（stderr）以獲取進度資訊。
- 在前端實作分頁（Tabs），將下載與轉檔功能分開。

**Non-Goals:**
- 實作自定義的 `ffmpeg` 濾鏡（如浮水印）。
- 實作影片剪輯功能。

## Decisions

- **解析 ffmpeg 進度**: `ffmpeg` 的進度資訊（如 `time=`）通常輸出到 stderr。我們將使用正則表達式解析任務進度。
- **前端 Tabs 結構**: 使用 Svelte 實作簡單的 Tab 切換，讓使用者可以在「下載」與「轉檔」之間切換。
- **批次任務處理**: 支援多個任務同時存在於列表，但預設限制並行執行的任務數量以避免過度佔用 CPU。

## Implementation Contract

- **行為 (Behavior)**: 使用者選擇檔案後，系統讀取中繼資料（如時長），啟動 `ffmpeg` 後開始解析 `time=` 欄位並計算百分比。
- **介面與數據 (Interface)**: 
  - Tauri Events: `transcode-progress` (payload: { id, progress, time })。
  - Rust 命令: `start_transcoding(tasks: Vec<Task>)`。
- **驗證標準 (Acceptance Criteria)**:
  - 能夠將 MOV 轉換為 MP4。
  - 進度條在 UI 上能即時更新。
  - 完成後觸發系統通知。

## Risks / Trade-offs

- [風險] ffmpeg 未安裝 → [對策] 啟動時檢查 ffmpeg 是否在 PATH 中，若無則提示使用者安裝。
- [風險] CPU 負載過高 → [對策] 限制同時轉檔的數量為 2 個。