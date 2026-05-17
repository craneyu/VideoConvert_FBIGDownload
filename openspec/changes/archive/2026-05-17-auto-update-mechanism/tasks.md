## 1. 環境配置 (Environment Setup)

- [x] 1.1 在 `src-tauri/Cargo.toml` 加入 `tauri-plugin-updater`，驗證方式：編譯通過。
- [x] 1.2 在 `tauri.conf.json` 配置 `updater` 區塊，設定 `endpoints` 為 GitHub 資源路徑，驗證方式：檢查 JSON 格式正確。

## 2. 後端實作 (Backend Implementation)

- [x] 2.1 在 `src-tauri/src/lib.rs` 初始化 `updater` 插件，驗證方式：確認 Rust 代碼無報錯。

## 3. 前端實作 (Frontend Implementation)

- [x] 3.1 在 Svelte 進入點加入更新檢查邏輯，若發現更新則顯示提示，驗證方式：模擬新版本存在時 UI 彈出提示。