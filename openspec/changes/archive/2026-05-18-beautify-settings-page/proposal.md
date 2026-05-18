## Why

目前的設定頁面功能已實裝，但視覺風格與主控制面板（Dashboard）尚有落差，缺乏 macOS 原生應用的精緻感。為了提供一致且高品質的使用者體驗，需要對設定頁面的 UI 進行深度美化與規範化。

## What Changes

- 佈局優化：調整容器間距、邊框圓角與陰影，使其符合 modern macOS 設計語言。
- 控制項美化：重新設計輸入框 (Input)、選擇框 (Select) 與勾選框 (Checkbox) 在不同狀態下的視覺表現 (Hover, Focus, Active)。
- 色彩與對比度：優化深色模式 (Dark Mode) 下的色彩分配，增加層次感並提升文字可讀性。
- 一致性調整：確保字體規格、圖示大小與間距標準與主程式完全一致。

## Capabilities

### New Capabilities

(無)

### Modified Capabilities

- settings-management: 更新設定介面的 UI 佈局與互動規格。
- enhanced-ui-experience: 將設定頁面的美化納入全域視覺規範。

## Impact

- Affected specs: 
    - openspec/specs/settings-management/spec.md
    - openspec/specs/enhanced-ui-experience/spec.md
- Affected code:
    - src/routes/settings/+page.svelte (主要修改對象)
    - src/app.css (可能新增全域控制項樣式)
