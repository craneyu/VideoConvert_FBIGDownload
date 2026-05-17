## Context

目前所有影片都直接存入下載資料夾根目錄。我們需要強化 `download_video` 指令，讓它能處理目錄結構的建立。

## Goals / Non-Goals

**Goals:**
- 實作 Rust 層的自動目錄建立邏輯。
- 更新 `yt-dlp` 的輸出路徑參數。

**Non-Goals:**
- 實作使用者自定義分類規則。

## Implementation Contract

- **行為 (Behavior)**: 使用者點擊下載後，Rust 端會先檢查並建立 `Downloads/VidBridge/{Source}` 目錄，然後將影片存入其中。
- **介面 (Interface)**: `download_video` 指令新增 `source: String` 參數。
- **驗證標準 (Acceptance Criteria)**: 下載後，檔案實際路徑必須包含 `VidBridge/{Source}` 段落。

## Risks / Trade-offs

- [風險] 權限問題無法建立目錄 → [對策] 捕獲 IO 錯誤並回傳給前端。