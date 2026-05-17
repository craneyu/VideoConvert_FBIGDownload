## 1. 後端參數擴展 (Backend Expansion)

- [x] 1.1 在 `src-tauri/src/commands/transcode.rs` 定義 `TranscodeOptions` 結構體並支援序列化，驗證方式：編譯通過且無警告。
- [x] 1.2 更新 `transcode_video` 指令以接收 `options` 參數，驗證方式：前端調用時參數能正確解構。
- [x] 1.3 實作參數至 `ffmpeg` arguments 的映射邏輯（包含 CRF, Preset, Scale, Codec），驗證方式：確認輸出的 `ffmpeg` 指令字串符合預期。

## 2. 前端設定介面 (Frontend UI)

- [x] 2.1 在 `src/routes/+page.svelte` 實作全域轉檔設定面板，包含預設選項（高品質/平衡/小檔案），驗證方式：點擊切換時狀態變更。
- [x] 2.2 實作「進階設定」展開區域，提供解析度與編碼選擇，驗證方式：切換「自定義」後可調整具體數值。
- [x] 2.3 更新 `startTranscode` 函式，將 UI 設定值傳遞至後端，驗證方式：發起轉檔請求時 Payload 包含正確設定。