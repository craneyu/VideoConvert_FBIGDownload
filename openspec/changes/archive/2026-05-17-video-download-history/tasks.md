## 1. 資料庫配置 (Database Setup)

- [x] 1.1 在 `src-tauri/Cargo.toml` 與 `package.json` 中加入 `tauri-plugin-sql` 相依性，驗證方式：執行 `npm install` 並確認編譯通過。
- [x] 1.2 在 `src-tauri/src/lib.rs` 初始化 SQL 插件，並實作 SQLite Persistence (SQLite 持久化)，建立 `download_history` 資料表，驗證方式：啟動程式後確認 `vidbridge.db` 檔案已產生且包含正確資料表。

## 2. 下載引擎實作 (Video Download Engine)

- [x] 2.1 在 Rust 端實作 Video Metadata Fetching (影片元數據獲取) 指令，使用 `yt-dlp --dump-json` 獲取影片標題，驗證方式：使用測試網址調用指令並確認回傳標題正確。
- [x] 2.2 在 `src-tauri/src/commands/download.rs` 實作 Progressive Downloading (進度下載) 指令，啟動 `yt-dlp` 並解析進度百分比，驗證方式：確認前端能接收到 `download-progress` 事件。
- [x] 2.3 實作下載完成後更新資料庫狀態為 'completed' 並紀錄檔案路徑，驗證方式：查詢 SQLite 確認紀錄已更新。

## 3. 前端介面與整合 (Download UI Integration)

- [x] 3.1 實作 Download Management UI (下載管理介面)，包含網址輸入與進度條組件，對接 `download_video` 指令，驗證方式：手動測試下載流程並確認進度條跳動。
- [x] 3.2 實作 History Retrieval (歷史紀錄讀取)，從 SQLite 讀取並顯示下載歷史列表，驗證方式：確認歷史列表能顯示下載成功的項目。
- [x] 3.3 實作 Local File Access (本地檔案存取) 功能，驗證方式：點擊「開啟資料夾」按鈕後 macOS Finder 能開啟目標路徑。
