## Context

VidBridge 目前處於規劃階段，僅有規格說明。我們需要建立實際的專案結構，以便開始後續「影片轉檔」與「下載」功能的開發。

## Goals / Non-Goals

**Goals:**
- 建立符合 Tauri 規範的專案目錄結構（Rust 後端 + Svelte 前端）。
- 配置基礎開發環境與相依性（Tailwind CSS）。
- 實作基本的主視窗管理與系統工具列 (System Tray) 入口。

**Non-Goals:**
- 實作具體的影片下載或轉檔核心邏輯。
- 實作複雜的介面排版或多國語系支援。

## Decisions

- **前端框架選擇 Svelte**: 考慮到 Svelte 的編譯特性與執行效率，與 Tauri 輕量化的目標一致，且開發體驗良好。
- **使用 tauri-plugin-log**: 為了方便開發除錯，將整合日誌插件。
- **系統工具列實作**: 使用 Tauri 內置的 SystemTray API，並在 Rust 層處理視窗切換邏輯，確保即使視窗關閉，程式仍能透過托盤常駐。

## Implementation Contract

- **行為 (Behavior)**: 程式啟動後應自動顯示主視窗，並在 macOS 工具列顯示應用程式圖示。圖示應支援右鍵選單應包含顯示/隱藏及結束選項。
- **介面與數據 (Interface)**: 
  - Rust 命令: `toggle_window` 用於控制主視窗顯示/隱藏。
  - Tauri 配置: `tauri.conf.json` 中需設定正確的標識符與視窗屬性。
- **驗證標準 (Acceptance Criteria)**:
  - 執行 `npm run tauri dev` 能夠成功編譯並開啟視窗。
  - 主視窗標題顯示為 "VidBridge"。
  - 關閉主視窗後，可透過工具列選單重新開啟，或選擇 "Quit" 結束程式。

## Risks / Trade-offs

- [風險] 開發環境配置衝突 → [對策] 嚴格按照 Tauri 官方文件初始化，並在 `package.json` 中鎖定版本。
