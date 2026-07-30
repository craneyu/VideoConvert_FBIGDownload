## 1. 設定層：theme 鍵與驗證（Rust）

- [x] 1.1 在 `src-tauri/src/commands/settings.rs` 既有的 `#[cfg(test)]` 模組新增失敗測試，斷言 `Settings::default()` 的 `theme` 為 `system`、`merge` 對 `light` 與 `dark` 生效、`merge` 對空字串與未知字串（例如 `sepia`）保留 `system`。此步驟交付的契約是 Invalid Theme Value Fallback 與 Default Settings Values 的可執行斷言。驗證：`cargo test` 出現預期的編譯錯誤或測試失敗，證明測試確實在檢查尚未存在的行為。
- [x] 1.2 依 design 決策「theme 值的驗證放在 Settings::merge，非法值回退 system」，為 `Settings` struct 新增 `theme` 欄位、`Default` 給 `system`、`merge` 只接受 `system`／`light`／`dark` 三值且非法值靜默保留預設。交付契約：Theme Setting Key 成立——`get_settings` 回報 `theme`，`update_setting` 維持通用 KV 寫入器不加 per-key 驗證，且既有資料庫無需 migration。驗證：1.1 的測試全部轉綠，`cargo test` 通過。

## 2. 樣式層：variant 與原生控制項

- [x] 2.1 依 design 決策「以 data-theme 屬性重新定義 dark: variant」，在 `src/app.css` 以 Tailwind v4 的 `@custom-variant` 把 `dark:` 從 `prefers-color-scheme` 媒體查詢改綁到 `<html>` 的 `data-theme="dark"`。交付契約：Resolved Theme Attribute Contract 的樣式端——呈現結果只由屬性決定，媒體查詢被完全繞過。驗證：`npm run dev` 後以 devtools 手動把 `<html>` 的 `data-theme` 在 `light` 與 `dark` 間切換，主畫面配色隨之改變，且此時將作業系統配色反向切換不影響畫面。
- [x] 2.2 依 design 決策「同步套用 color-scheme 讓原生控制項跟隨主題」，在 `src/app.css` 依 `data-theme` 屬性選擇器宣告對應的 `color-scheme`。交付契約：Native Control Color Scheme 成立——原生繪製的 UI 跟隨當前主題而非作業系統偏好。驗證：在系統為深色、`data-theme` 手動設為 `light` 的情況下，開啟主畫面轉檔選項的原生下拉選單，選單本身為淺色外觀；捲軸同樣為淺色。

## 3. 主題模組：解析與套用

- [x] 3.1 依 design 決策「集中主題解析與套用邏輯於 src/lib/theme.ts」，新增該模組，匯出純函式將設定模式解析為具體配色（未知值與 `system` 走同一路徑）、將結果寫入 `<html>` 的 `data-theme`、並提供訂閱與解除訂閱作業系統配色變更的能力。交付契約：Resolved Theme Attribute Contract——`data-theme` 恆為 `light` 或 `dark`，絕不為 `system`、空字串或缺席。驗證：`npm run check` 通過；並在 devtools console 直接呼叫解析函式，確認傳入 `light`／`dark`／`system`／`sepia` 四種輸入分別得到 `light`／`dark`／依系統偏好／與 `system` 相同的結果。
- [x] 3.2 [P] 在 `src/lib/stores/settings.svelte.ts` 的 `Settings` interface 新增 `theme` 欄位，型別為 `system`、`light`、`dark` 三個字面值的聯集。交付契約：前後端設定形狀一致，`settingsStore.update('theme', ...)` 具備型別保護。驗證：`npm run check` 通過，且對 `theme` 傳入非法字面值時 svelte-check 回報型別錯誤。
- [x] 3.3 在 `src/routes/+layout.svelte` 以單一 `$effect` 觀察 `settingsStore.settings.theme` 並呼叫主題模組套用，同時在切離 `system` 時解除作業系統配色訂閱。交付契約：Theme Mode Selection 的即時生效（因 `settingsStore.update` 為樂觀更新，畫面在 IPC 往返完成前就改變）與 Following the Operating System Preference（`system` 模式跟隨、固定模式不跟隨）。驗證：手動在設定頁切換三個選項，畫面即時改變；停在 `system` 時改變系統配色，App 不重啟即跟隨；改選 `light` 後再改變系統配色，App 維持淺色不被覆寫。

## 4. 首次繪製前套用

- [x] 4.1 依 design 決策「以 localStorage 作為首次繪製前的同步主題快取」，在主題套用路徑寫入 localStorage 快取（存的是設定模式，可為 `system`，非解析後的值），並於 `settingsStore` 載入完成後以 SQLite 權威值重新套用與刷新快取。交付契約：Theme Applied Before First Paint 的快取一致性——快取只在首繪前具權威性，分歧的影響上限是數百毫秒的舊主題。驗證：手動將快取改為與資料庫不同的值後重新載入，畫面先套用快取值、隨即被 SQLite 值修正，且快取內容被更新為 SQLite 值。
- [x] 4.2 在 `src/app.html` 的 `<head>` 內嵌同步 script，依「讀快取 → 退回 `prefers-color-scheme` → 寫入 `data-theme`」的最小順序在首次繪製前套用主題。交付契約：Theme Applied Before First Paint——深色情境啟動不出現白色閃屏；且快取不可用或值非法時退回系統偏好而不拋出中斷頁面載入的例外。驗證：系統設為深色、`theme` 設為 `dark`，重新啟動 App 全程無白色畫面；再以 devtools 將快取寫成 `sepia` 後重新載入，畫面套用系統偏好且 console 無未捕捉例外。

## 5. 設定頁控制項

- [x] 5.1 在 `src/routes/settings/+page.svelte` 新增三段式主題選擇控制項，選項文案為「跟隨系統」、「淺色」、「深色」，並沿用該頁既有的互動回饋樣式。交付契約：Theme Mode Selection 的使用者入口——選擇後立即生效且重啟後保持，首次啟動（資料庫無 `theme` 鍵）等同「跟隨系統」，與本變更前行為一致。驗證：三個選項各選一次，畫面即時改變；每次選擇後重啟 App，選擇被保留；刪除資料庫 `settings` 表中的 `theme` 列後重啟，App 回到跟隨系統的行為。

## 6. 整體驗收

- [x] 6.1 執行完整驗收：`cargo test` 與 `npm run check` 皆通過；並依 design 的 Implementation Contract 逐條走過手動驗證矩陣（三段式切換即時生效與持久化、`system` 即時跟隨與固定模式不跟隨、深色啟動無閃屏、淺色下原生下拉選單為淺色外觀、非法快取值回退）。交付契約：本變更所有需求可觀察地成立。驗證：上述每一項手動斷言逐條確認通過，任一項失敗則回到對應任務修正。
- [x] 6.2 記錄實作過程中發現但不在本變更範圍內的淺色模式對比瑕疵（若有）。交付契約：Non-Goals 的邊界被實際遵守，發現的問題不被靜默吞掉也不擴張本變更。驗證：於本變更目錄留下一份條列清單，或在確認無瑕疵時明確記錄「無」。
