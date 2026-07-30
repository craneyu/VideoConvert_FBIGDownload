## Why

App 目前只能跟隨作業系統的配色偏好，使用者無法在 App 內選擇淺色或深色。淺色樣式其實早已完整存在（主畫面與設定頁共有 62 處 `dark:` variant，基底類別如 `bg-neutral-50` 就是淺色），但 Tailwind v4 的 `dark:` variant 預設綁定 `prefers-color-scheme` 媒體查詢，因此在深色系統上永遠只會呈現深色，使用者沒有任何覆寫手段。

這是既有樣式資產無法被使用的問題，不是需要重新設計配色的問題，投入成本低而使用者可感知度高。

## What Changes

- 新增 `theme` 設定鍵，可選值為 `system`、`light`、`dark`，預設為 `system`，藉此保留現有使用者的行為不變。
- 在 `src/app.css` 以 Tailwind v4 的 `@custom-variant` 重新定義 `dark:` variant，使其改為由 `<html>` 上的 `data-theme="dark"` 屬性驅動，而非媒體查詢。
- 由前端在啟動時與設定變更時，將解析後的實際配色（`light` 或 `dark`）寫入 `<html>` 的 `data-theme` 屬性。
- 於 `src/app.html` 內嵌一段同步執行的 script，在首次繪製前就套用 `data-theme`，避免深色使用者每次啟動時看到白色閃屏（FOUC）。
- 設定頁新增三段式主題選擇控制項。
- 當 `theme` 為 `system` 時，監聽作業系統配色變更並即時跟隨，無須重啟 App。

此變更不含 BREAKING 項目：新增設定鍵有預設值，既有資料庫無需 migration（`settings` 表為 key-value 結構）。

## Capabilities

### New Capabilities

- `theme-switching`: 主題模式的三段式選擇、`system` 模式的解析與即時跟隨、`data-theme` 屬性契約，以及首次繪製前套用以避免閃屏。

### Modified Capabilities

- `settings-management`: 「Default Settings Values」需求的預設值對照表新增 `theme` 鍵及其預設值 `system`；設定結構新增對應欄位與解析邏輯。

## Impact

- Affected specs:
  - 新增：`theme-switching`
  - 修改：`settings-management`
- Affected code:
  - New:
    - src/lib/theme.ts
  - Modified:
    - src/app.css
    - src/app.html
    - src/routes/+layout.svelte
    - src/routes/settings/+page.svelte
    - src/lib/stores/settings.svelte.ts
    - src-tauri/src/commands/settings.rs
  - Removed:
    - (none)
- Affected dependencies: 無新增套件。Tailwind 已為 v4.3，`@custom-variant` 為其內建指示詞。
- 注意：專案根目錄的 Tailwind 設定檔在 Tailwind v4 下未被載入（`src/app.css` 未使用 `@config` 指示詞），因此主題設定不可寫在該檔案中，否則會靜默失效。
