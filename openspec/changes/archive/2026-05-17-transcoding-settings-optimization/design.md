## Context

此設計旨在將 `ffmpeg` 的強大功能透過簡單的 UI 暴露給使用者。我們需要處理從前端傳入的選項結構，並在 Rust 層安全地轉換為命令列參數。

## Goals / Non-Goals

**Goals:**
- 實作 `TranscodeOptions` 結構體與參數映射邏輯。
- 在前端實作美觀且易用的設定面板。
- 確保自定義解析度不會破壞影片比例（使用 `scale=-2:height`）。

**Non-Goals:**
- 實作音訊編碼細節設定。
- 實作影片雙語軌選擇。

## Decisions

- **預設與自定義共存**: 若使用者選擇預設組合，進階設定將被自動填入對應值但保持禁用狀態，除非手動切換至「自定義」。
- **使用 CRF 控制品質**: 對於 H.264/H.265，我們將優先使用 CRF（Constant Rate Factor）而非固定比特率，以獲得更好的視覺品質。

## Implementation Contract

- **介面 (Interface)**: 
  - Rust 結構 `TranscodeOptions { preset: String, resolution: String, codec: String }`。
  - 命令 `transcode_video` 更新為接收此結構。
- **驗證標準 (Acceptance Criteria)**:
  - 選擇 「Small Size」時，輸出的檔案體積應顯著小於「High Quality」。
  - 選擇 H.265 時，`ffprobe` 應顯示視訊流編碼為 `hevc`。

## Risks / Trade-offs

- [風險] H.265 編碼時間過長 → [對策] 在 UI 提示使用者 H.265 雖然檔案更小但轉檔較慢。