## Context

目前的設定頁面實作了基本的 Svelte 佈局，但視覺細節與主頁面的 Dashboard 相比顯得較為簡陋。專案目標是建立一個具有高品質 macOS 原生質感的影音工具，因此需要優化設定頁面的 UI 以確保整體品牌形象的一致性。

## Goals / Non-Goals

**Goals:**

- 統一設計語彙：使用與 Dashboard 相同的 Tailwind 色票、圓角標準與間距標準。
- 強化視覺層次：利用背景色差與邊框細節，在深色與淺色模式下都能清晰區分內容區塊。
- 提升互動反饋：確保所有表單元件在 Hover 與 Focus 狀態下都有明顯的視覺提示。

**Non-Goals:**

- 不增加新的設定功能或後端邏輯。
- 不修改導航架構或現有的設定存取方式。

## Decisions

### 採用容器化佈局標準

為了與主頁面保持一致，設定項目將分組放置在具有白色或深灰色背景的圓角容器中，並添加細微的邊框與陰影。
- 背景與邊框：淺色模式使用 bg-white 搭配 border-neutral-100；深色模式使用 bg-neutral-900 搭配 border-neutral-800。
- 圓角與陰影：統一使用 rounded-2xl 與 shadow-sm。

### 優化表單控制項樣式

所有的 input、select 與 checkbox 將進行視覺升級：
- Focus 狀態：統一增加藍色 focus ring 與邊框顏色變化。
- 互動過渡：所有視覺變化應包含 transition-all 以確保流暢感。

### 強化深色模式對比

針對深色模式下的背景，容器應使用稍微淺一點的黑色，並使用較暗的邊框色，以建立深度感。

## Implementation Contract

- 行為表現：使用者切換至設定頁面時，佈局規格應與主頁面完全對應，無突兀的跳躍感。
- 資料與介面：本設計僅涉及 src/routes/settings/+page.svelte 的 UI 實作，不涉及 IPC 或資料結構更動。
- 驗證標準：
    - 淺色模式下：容器與背景有足夠對比度。
    - 深色模式下：文字與背景有良好對比。
    - 響應式與間距：在不同視窗大小下，最大寬度應限制在 max-w-2xl 並保持水平置中。
- 範圍界限：僅限於設定頁面的視覺修改。

## Risks / Trade-offs

- [Risk] -> 過度裝飾可能導致渲染壓力。
- [Mitigation] -> 使用標準的 CSS/Tailwind 屬性。
