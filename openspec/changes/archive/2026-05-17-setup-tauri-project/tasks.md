## 1. 專案基礎環境建立 (Project Foundation)

- [x] 1.1 實作 Tauri Project Structure，初始化專案目錄並確保前端框架選擇 Svelte，驗證方式：執行 `npm run tauri dev` 確認主視窗能成功啟動。
- [x] 1.2 配置 Development Dependencies，包含 Tailwind CSS 整合與必要插件，驗證方式：在前端組件套用 Tailwind 類別並確認樣式渲染正確。
- [x] 1.3 在 Rust 後端完成使用 tauri-plugin-log 的初始化，驗證方式：啟動程式後在終端機確認有日誌輸出。

## 2. 視窗管理實作 (App Window Management)

- [x] 2.1 實作 Main Window Display，配置 `tauri.conf.json` 確保主視窗標題顯示為 "VidBridge"，驗證方式：手動啟動應用程式並檢查視窗標題列。
- [x] 2.2 確保 Dark Mode Support 正常運作，驗證方式：切換 macOS 系統主題（淺色/深色）並確認應用程式介面背景色隨之變動。

## 3. 系統托盤功能實作 (System Tray Interaction)

- [x] 3.1 實作系統工具列實作 (System Tray Icon)，確保 VidBridge 圖示常駐於 macOS 工具列，驗證方式：啟動程式後檢查上方工具列是否出現專屬圖示。
- [x] 3.2 實作 System Tray Menu 選單邏輯，包含 "Show/Hide Window"、"Settings" 與 "Quit" 選項，驗證方式：右鍵點擊托盤圖示確認選單內容與文字正確。
- [x] 3.3 實作 Background Execution 與視窗切換邏輯，當主視窗關閉時程式不退出且可透過托盤重新顯示，驗證方式：關閉視窗後再次從托盤選單點擊 "Show" 並確認視窗重新出現。
