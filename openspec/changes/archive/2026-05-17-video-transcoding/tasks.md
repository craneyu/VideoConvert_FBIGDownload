## 1. 後端轉檔引擎實作 (Transcoding Engine)

- [x] 1.1 在 `src-tauri/src/commands/transcode.rs` 實作 `ffmpeg` 執行邏輯，驗證方式：手動執行指令並確認能產生正確的輸出檔案。
- [x] 1.2 實作 `ffmpeg` stderr 解析邏輯，獲取 `time=` 並計算百分比，驗證方式：確認前端能接收到正確的 `transcode-progress` 事件。
- [x] 1.3 整合系統通知功能，當轉檔任務完成時發送通知，驗證方式：任務完成後確認 macOS 通知中心彈出訊息。

## 2. 前端介面調整 (Frontend UI)

- [x] 2.1 在 `src/routes/+page.svelte` 實作 Tab 切換功能（下載 / 轉檔），驗證方式：點擊 Tab 能正確切換介面且內容不遺失。
- [x] 2.2 實作轉檔任務列表與檔案選擇器（支援拖放），驗證方式：拖入檔案後列表正確顯示新任務。
- [x] 2.3 實作轉檔進度條與狀態顯示，驗證方式：執行轉檔時進度條應隨之跳動。

## 3. 系統整合 (System Integration)

- [x] 3.1 實作啟動時檢查 `ffmpeg` 是否安裝，驗證方式：若環境中無 `ffmpeg` 則顯示警告訊息。
