## Context

App 使用 Tailwind v4.3（`src/app.css` 僅有 `@import "tailwindcss";`）。淺色與深色兩套樣式皆已完整存在——基底類別為淺色（例如主畫面根容器的 `bg-neutral-50`），`dark:` variant 提供深色覆寫，主畫面與設定頁合計 62 處。問題純粹在於 Tailwind v4 的 `dark:` variant 預設綁定 `prefers-color-scheme` 媒體查詢，App 內沒有覆寫手段。

現有設定基礎設施可直接沿用：`settings` 為 key-value 資料表，Rust 端有 `Settings` struct、`Default` 實作與 `merge` 方法，並提供 `get_settings` 與 `update_setting` 兩個 IPC 命令；`update_setting` 是通用 KV 寫入器，不含任何 per-key 驗證。前端有 `settingsStore`，`update` 方法採樂觀更新（先改本地狀態再送 IPC，失敗則回滾）。

關鍵約束：`get_settings` 是非同步 IPC，且 `settingsStore.load()` 具備最多 10 次、每次間隔 1 秒的資料庫重試邏輯。這代表主題的權威值在首次繪製時**必定尚未取得**。

## Goals / Non-Goals

**Goals:**

- 使用者可在設定頁選擇 `system`、`light`、`dark` 三種主題模式，選擇後立即生效且重啟後保持。
- 預設值為 `system`，確保既有使用者升級後行為完全不變。
- 深色使用者啟動 App 時不得出現白色閃屏。
- `system` 模式下，作業系統配色變更時 App 即時跟隨，無須重啟。
- 原生控制項（設定頁與主畫面的下拉選單、捲軸）配色跟隨當前主題。

**Non-Goals:**

- 不調整任何既有配色值。本變更只讓已存在的淺色樣式變得可達，不重新設計色板；`enhanced-ui-experience` 既有的深色對比需求不受影響。
- 不提供自訂主題色、強調色或第三種配色方案。
- 不提供依時間自動切換（日出日落排程）。
- 不將主題狀態同步到 Rust 端供原生視窗裝飾使用；本變更範圍內主題僅影響 webview 內容。
- 不補齊 `dark:` variant 覆蓋不全之處。若實作過程發現某元件在淺色模式下對比不足，記錄為後續問題，不在本變更修正。
- 不引入前端測試框架。專案目前沒有任何前端測試 runner（`package.json` 無 vitest，亦無測試檔案），在主題切換這個範圍內建立測試基礎設施屬於範圍膨脹，應獨立成一個變更。因此前端邏輯的驗證以純函式抽出、`npm run check` 型別檢查與明確的手動斷言涵蓋。

## Decisions

### 以 data-theme 屬性重新定義 dark: variant

在 `src/app.css` 以 Tailwind v4 的 `@custom-variant` 指示詞，將 `dark:` 從媒體查詢改綁到 `<html>` 上的 `data-theme="dark"` 屬性：

    @import "tailwindcss";
    @custom-variant dark (&:where([data-theme="dark"], [data-theme="dark"] *));

前端一律將解析後的具體值（`light` 或 `dark`）寫入 `data-theme`，不留空。如此媒體查詢被完全繞過，呈現結果只由屬性決定，行為可預期。

**替代方案：** Tailwind v3 慣用的 `darkMode: 'class'` 加 `.dark` class。已否決——Tailwind v4 在 `src/app.css` 未使用 `@config` 指示詞的情況下**不會載入專案根目錄的 Tailwind 設定檔**，寫在該檔案的設定會靜默失效，這是本變更最容易踩的陷阱。選用屬性而非 class，是因為屬性不會與 Tailwind 自身產生的 utility class 命名空間相混。

### 以 localStorage 作為首次繪製前的同步主題快取

主題的權威儲存是 SQLite，但讀取路徑是非同步 IPC，無法在首次繪製前完成。因此採雙層來源：

- **SQLite**：權威值，經 `get_settings` 與 `update_setting` 讀寫。
- **localStorage**：同步快取，僅供首次繪製前使用。每次主題設定變更時同步寫入。

`src/app.html` 的 `<head>` 內嵌一段同步 script，依序嘗試：讀取 localStorage 快取 → 若無則讀取 `prefers-color-scheme` → 立即寫入 `data-theme`。稍後 `settingsStore` 載入完成時，若 SQLite 值與快取不一致，則以 SQLite 值重新套用並刷新快取。

一致性風險（快取與權威值分歧）的影響上限是「首次繪製套用舊主題，數百毫秒後修正」，屬可接受的視覺瑕疵，不影響資料正確性。

**替代方案一：** 接受閃屏，不做快取。已否決——深色使用者每次啟動都看到白色閃光，是使用者可感知的品質缺陷。
**替代方案二：** 用 Tauri 的 webview 初始化 script，在視窗建立前由 Rust 端讀 DB 注入。已否決——`get_settings` 依賴 tauri-plugin-sql 的連線池狀態，該池在視窗建立時尚未必就緒（前端重試邏輯的存在正是此事的證據），時序過於脆弱。

### theme 值的驗證放在 Settings::merge，非法值回退 system

`update_setting` 是通用 KV 寫入器，加入 per-key 驗證會破壞其單一職責。因此 `theme` 的合法值檢查放在 `Settings::merge`：資料庫值為 `system`、`light`、`dark` 之一時採用，否則保留預設值 `system`。

這讓手動改資料庫、舊版本殘留值、或未來移除某個模式等情境都能安全降級，且是純函式，可用 Rust 單元測試涵蓋。前端 `src/lib/theme.ts` 亦對未知值做同樣的 `system` 回退，形成雙層防護。

### 集中主題解析與套用邏輯於 src/lib/theme.ts

新增模組 `src/lib/theme.ts`，承擔三件事：將主題模式解析為具體配色、將結果套用到 `<html>`、在 `system` 模式下訂閱作業系統配色變更。`src/routes/+layout.svelte` 只以一個 `$effect` 觀察 `settingsStore.settings.theme` 並呼叫此模組。

如此 `system` 的解析規則只有一份實作，設定頁與主畫面都不需要自行判斷。因為 `settingsStore.update` 採樂觀更新，`$effect` 會在 IPC 往返完成前就觸發，使用者點選後即時看到變化。

訂閱必須在切離 `system` 時解除，否則作業系統配色變更會覆寫使用者明確選擇的固定主題。

### 同步套用 color-scheme 讓原生控制項跟隨主題

僅設定 `data-theme` 不會影響瀏覽器繪製的原生 UI。主畫面的轉檔選項有兩個原生 `<select>`，設定頁亦有一個，捲軸同理；若不處理，淺色模式下會出現深色的原生下拉選單。

因此 `data-theme` 套用時，同時設定 CSS 的 `color-scheme` 屬性為對應值（`light` 或 `dark`），由 `src/app.css` 依屬性選擇器提供。

## Implementation Contract

**Behavior:**

- 設定頁出現主題選擇控制項，三個選項為「跟隨系統」、「淺色」、「深色」。
- 選擇後畫面立即改變，不需重新載入或重啟；重啟 App 後保持該選擇。
- 選「跟隨系統」時，改變 macOS 或 Windows 的系統配色，App 在不重啟的情況下跟著改變。
- 選「淺色」或「深色」時，改變系統配色不影響 App。
- 首次啟動（資料庫無 `theme` 鍵）等同「跟隨系統」，與本變更前的行為一致。
- 深色情境下啟動 App，不出現白色閃屏。

**Interface / data shape:**

- 設定鍵名 `theme`，值為字串，合法值 `system`、`light`、`dark`，預設 `system`。
- Rust 端 `Settings` struct 新增 `theme: String` 欄位，並納入 `Default` 與 `merge`。既有 IPC 命令 `get_settings` 與 `update_setting` 的簽章不變。
- 前端 `Settings` interface 新增 `theme` 欄位，型別為三個字面值的聯集。
- DOM 契約：`<html>` 的 `data-theme` 屬性值恆為 `light` 或 `dark`，絕不為 `system`、空字串或缺席。`system` 是設定層的模式名稱，不是 DOM 層的值。
- localStorage 快取鍵名為固定字串，儲存的是設定模式（可為 `system`），非解析後的值——如此使用者選擇跟隨系統時，下次啟動仍會重新依當下系統配色解析。

**Failure modes:**

- 資料庫中 `theme` 值不在合法集合內：`merge` 保留預設 `system`，不回報錯誤、不寫回資料庫。
- localStorage 不可用或內容非法：內嵌 script 退回讀取 `prefers-color-scheme`，不拋出例外中斷頁面載入。
- `update_setting` 失敗：沿用 `settingsStore.update` 既有的回滾行為，本地狀態復原，畫面隨 `$effect` 回到原主題；錯誤僅記錄於 console，不彈出對話框。
- `get_settings` 在重試耗盡後仍失敗：`settings` 保持 null，主題維持內嵌 script 套用的結果，App 仍可正常使用。

**Acceptance criteria:**

- Rust 單元測試（`cargo test`，沿用 `src-tauri/src/commands/settings.rs` 既有的 `#[cfg(test)]` 模組）：`Settings::default()` 的 `theme` 為 `system`；`merge` 對 `light` 與 `dark` 生效；`merge` 對非法值（空字串與任意未知字串）保留 `system`。
- 型別驗證：`npm run check` 通過，確認前端 `Settings` interface 的 `theme` 欄位型別與設定頁控制項一致。
- 手動驗證（非法值回退）：以 devtools 在 localStorage 寫入未知值後重新載入，畫面套用系統偏好而非拋錯；此路徑因無前端測試 runner 而以手動斷言涵蓋。
- 手動驗證（三段式選項各一次）：切換後畫面即時變化；重啟後保持；`system` 模式下切換系統配色會即時跟隨，固定模式下不跟隨。
- 手動驗證（閃屏）：系統設為深色、`theme` 設為 `dark` 或 `system`，重新啟動 App，啟動過程不出現白色畫面。
- 手動驗證（原生控制項）：淺色模式下開啟主畫面轉檔選項的下拉選單，選單本身為淺色外觀。

**Scope boundaries:**

- **In scope**：`theme` 設定鍵的儲存與驗證、`dark:` variant 的重新定義、`data-theme` 與 `color-scheme` 的套用、首繪前的內嵌 script 與 localStorage 快取、`system` 模式的即時跟隨、設定頁的三段式控制項。
- **Out of scope**：任何既有配色值的調整、`dark:` variant 覆蓋不全的補齊、原生視窗裝飾的主題化、主畫面新增主題快捷切換入口。

## Risks / Trade-offs

- **雙來源（SQLite 與 localStorage）可能分歧** → 快取只在首次繪製前具權威性，`settingsStore` 載入完成後一律以 SQLite 值覆寫並刷新快取；分歧的影響上限是數百毫秒的舊主題。
- **`@custom-variant` 寫錯會讓 62 處 `dark:` 全部靜默失效** → 屬性選擇器一旦寫錯，症狀是「深色模式完全沒有深色樣式」，極為明顯；驗收步驟包含三段式選項各切一次，可立即發現。
- **切離 `system` 時忘記解除系統配色訂閱** → 症狀是使用者選了固定主題後，改系統配色仍會被覆寫。驗收條件已明確列出「固定模式下不跟隨」這一項。
- **內嵌 script 讓 `src/app.html` 出現行為邏輯** → 這是為了避免閃屏必須付的代價。將邏輯限制在「讀快取、退回媒體查詢、寫屬性」的最小範圍，其餘判斷一律留在 `src/lib/theme.ts`。
- **深色使用者長期未實際檢視淺色模式，淺色下可能存在既有對比瑕疵** → 已明確列為 Non-Goal；實作時若發現則記錄為後續問題，不擴張本變更範圍。
