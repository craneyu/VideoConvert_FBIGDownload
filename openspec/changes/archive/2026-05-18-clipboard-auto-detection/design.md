## Context

目前 VidBridge 的下載流程完全依賴手動貼上。為了優化 UX，我們需要一種方式在不造成效能負擔的前提下，自動偵測剪貼簿中的影片連結。Tauri 提供了讀取剪貼簿的 API，前端可以透過聚焦事件 (Focus Event) 來觸發檢查。

## Goals / Non-Goals

**Goals:**

- 實作主動偵測：在使用者回到 App 時自動檢查剪貼簿。
- 提供直覺 UI：以提示條的形式詢問使用者是否使用偵測到的連結。
- 可配置性：使用者可以在設定中關閉此功能。

**Non-Goals:**

- 不實作全時背景監控（避免隱私疑慮與效能損耗）。
- 不會自動開始下載（必須經過使用者確認）。

## Decisions

### 採用事件驅動的偵測機制

我們不使用無限迴圈監控剪貼簿，而是透過 Svelte 的 window.onfocus 事件。
- 理由：這能確保只有在使用者「想要」操作 App 時才執行檢查，且符合作業系統節能規範。

### 嚴格的網址 Regex 過濾

使用正規表達式精準識別 Facebook 與 Instagram 的影片連結。
- 理由：避免使用者複製了一般網頁網址時也被提示。

### 狀態管理整合

將「剪貼簿偵測開關」整合進 settingsStore。
- 理由：確保設定能即時生效且跨 Session 持久化。

## Implementation Contract

- 行為表現：當 window focus 且 settings.detect_clipboard 為 true 時，若剪貼簿有 FB/IG 連結且不等於目前輸入框內容，則顯示提示。
- 介面規範：
    - 新增 IPC 指令 read_clipboard_text (由 Rust 提供安全存取)。
    - 前端新增 ClipboardBanner 元件。
- 驗證標準：
    - 複製非影片網址：不應出現提示。
    - 複製影片網址並進入 App：應顯示提示條。
    - 點選「使用此連結」：提示條消失，網址填入輸入框。
- 範圍界限：僅影響「影片下載」分頁的 URL 輸入邏輯。

## Risks / Trade-offs

- [Risk] -> 使用者頻繁切換視窗可能導致重複提示。
- [Mitigation] -> 紀錄「最後一次偵測到的網址」，若內容未變則不重複提示。
