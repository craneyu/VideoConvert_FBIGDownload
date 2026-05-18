## Why

目前使用者下載影片需要手動複製網址、切換視窗、點選輸入框並貼上。透過實作剪貼簿自動偵測，系統可以主動識別符合條件的 FB/IG 網址，大幅縮短操作流程，提升產品的「聰明感」與便利性。

## What Changes

- 剪貼簿監控機制：實作一個背景監控邏輯（或在視窗聚焦時觸發），偵測剪貼簿內容是否包含 Facebook 或 Instagram 的影片網址。
- UI 智慧提示/自動填入：當偵測到有效網址時，在主介面顯示提示條 (Toast/Banner) 或自動填入輸入框。
- 使用者偏好控制：在設定頁面新增「自動偵測剪貼簿網址」開關，允許使用者自行決定是否開啟此功能。

## Capabilities

### New Capabilities

- clipboard-monitoring: 負責偵測系統剪貼簿內容並過濾有效影片網址的邏輯。

### Modified Capabilities

- settings-management: 新增儲存「是否開啟剪貼簿監控」的設定選項。
- download-ui-integration: 擴展下載介面，以支援來自剪貼簿偵測的網址輸入流。

## Impact

- Affected specs: 
    - openspec/specs/clipboard-monitoring/spec.md
    - openspec/specs/settings-management/spec.md
    - openspec/specs/download-ui-integration/spec.md
- Affected code:
    - src-tauri/src/lib.rs: 註冊或實作剪貼簿讀取指令。
    - src/routes/+page.svelte: 處理偵測到的網址並顯示 UI 提示。
    - src/lib/stores/settings.svelte.ts: 擴展 settings 介面與預設值。
