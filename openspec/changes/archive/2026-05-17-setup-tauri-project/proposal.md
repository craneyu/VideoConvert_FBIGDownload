## Why

目前 VidBridge 僅有規格書，尚未建立開發環境。此項變更旨在建立基於 Tauri 的基礎專案架構，為後續「影片轉檔」與「社群影片下載」功能提供實作的開發環境。

## What Changes

- 初始化 Tauri 專案架構（包含 Rust 後端與 Svelte 前端環境）。
- 配置基礎開發相依性（如 Tailwind CSS）。
- 實作基本的視窗顯示與系統工具列 (System Tray) 入口，確保程式能常駐執行。

## Capabilities

### New Capabilities

- `project-foundation`: 專案基礎架構、編譯流程與開發環境配置。
- `app-window-management`: macOS 主視窗的生命週期管理、深淺色模式支援。
- `system-tray-interaction`: macOS 系統工具列 (Menu Bar) 的圖示顯示與基本選單入口。

### Modified Capabilities

(none)

## Impact

- Affected specs: `project-foundation`, `app-window-management`, `system-tray-interaction`
- Affected code:
  - New: `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`, `src/App.svelte`, `package.json`, `tailwind.config.js`
