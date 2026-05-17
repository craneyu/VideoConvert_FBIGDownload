## 1. 核心邏輯實作 (Core Implementation)

- [x] 1.1 更新 `src-tauri/src/commands/download.rs` 的 `download_video` 指令，新增 `source` 參數並實作 Source-Based Directory Creation (來源目錄建立) 邏輯，驗證方式：確認編譯通過。
- [x] 1.2 在 Rust 端使用 `std::fs::create_dir_all` 確保路徑存在，並將其應用於 `yt-dlp` 的 `-o` 參數，實作 Categorized File Saving (分類檔案儲存)，驗證方式：手動下載後檢查實體目錄。

## 2. 前端整合 (Frontend Integration)

- [x] 2.1 更新 `src/routes/+page.svelte`，在呼叫 `download_video` 時傳遞正確的 `source` 字串，驗證方式：確認下載任務能正確啟動且分類正確。