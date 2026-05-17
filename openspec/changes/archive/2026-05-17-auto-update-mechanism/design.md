## Context

我們將使用 Tauri 官方的 updater 插件來實作。這需要配置 GitHub 託管的更新 JSON 檔案路徑。

## Goals / Non-Goals

**Goals:**
- 實作啟動時自動檢查更新。
- 實作前端更新提示對話框。

**Non-Goals:**
- 實作強制更新（使用者可選擇稍後安裝）。

## Implementation Contract

- **行為 (Behavior)**: 啟動時若發現新版本，介面應彈出提示。
- **驗證標準 (Acceptance Criteria)**: 修改本地版本號模擬舊版本，確認能否正確偵測到 GitHub 上的最新版（若有）。

## Risks / Trade-offs

- [風險] 簽署金鑰遺失 → [對策] 需妥善保管私鑰，否則無法發布後續更新。