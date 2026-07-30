## 1. 音訊不再二次壓縮（獨立於視訊策略，可先交付）

- [x] 1.1 讓重新編碼路徑依探測到的音訊 codec 決定音訊處理：已是 AAC 則直接複製、非 AAC 則轉為 AAC、無音訊軌則不產生任何音訊參數。交付 `Audio Is Copied When Already AAC`，並落實 design 決策「音訊在來源已是 AAC 時直接複製」。實作位置為 `src-tauri/src/commands/download.rs` 的後處理參數組裝；判斷所需的音訊 codec 欄位已存在於既有的探測結果結構中。驗證：新增單元測試以 spec 的「audio decision by probed codec」表格四列為輸入（aac→複製、none→無參數、opus→轉碼、mp3→轉碼），斷言產生的 ffmpeg 音訊參數。

## 2. 平台編解碼能力偵測

- [x] 2.1 新增一個模組，對外提供「本平台能否解碼指定視訊 codec」的三態查詢（可以／不行／未知），codec 名稱比較不分大小寫，且不支援的平台走回傳「未知」的預設實作。交付 `Three-State Codec Decode Capability`，並落實 design 決策「平台能力偵測回傳三態，只有明確的「可以」算支援」。實作位置為新檔 `src-tauri/src/commands/codec_support.rs`，並於 `src-tauri/src/commands/mod.rs` 註冊。驗證：新增單元測試斷言同一 codec 以 `AV1`／`av1`／`Av1` 查詢得到相同結果，且預設實作回傳「未知」而非「可以」。
- [x] 2.2 讓查詢在單一 process 內只實際詢問平台一次，之後重用結果。交付 `Capability Is Queried Once Per Process`，並落實 design 決策「偵測結果於 process 內只查一次」。驗證：新增單元測試以一個可計數的假平台查詢注入，斷言重複查詢同一 codec 時底層只被呼叫一次。
- [x] 2.3 讓平台查詢失敗（框架載入失敗、呼叫回傳錯誤、panic）一律視為「未知」而非「可以」，且不讓下載因此失敗。交付 `Detection Failure Is Treated As Unknown`。驗證：新增單元測試以一個必定失敗的假查詢注入，斷言結果為「未知」且不回傳錯誤給呼叫端。
- [x] 2.4 實作 macOS 分支：以 VideoToolbox 的硬體解碼查詢回答，並於 `src-tauri/Cargo.toml` 或建置設定加入該系統框架的連結。交付 design 決策「macOS 用 VideoToolbox 的硬體解碼查詢，並接受它低估軟體解碼」，且該低估行為須寫在程式碼註解裡。驗證：在 macOS 上手動查詢 AV1 並與 `VTIsHardwareDecodeSupported` 的獨立驗證結果一致（本機 M4／macOS 26.6 實測為支援）。
- [x] 2.5 [P] 讓除 macOS 以外的平台走同一個回傳「未知」的預設實作，因而一律走重新編碼並維持本變更前的行為。交付 design 決策「Windows 與 Linux 現階段一律回「未知」」。程式碼註解須寫明 Windows 是刻意延後而非遺漏，並指出原因（`MFTEnumEx` 為 COM API 需額外依賴，且此專案的 Windows target 無法在 macOS 編譯 —— 實測會在 `ring` 的 C 編譯階段因找不到 `assert.h` 而失敗）。驗證：新增單元測試斷言預設實作對任何 codec 皆回傳「未知」；並在 macOS 上執行 `cargo test` 確認 macOS 分支與預設分支不會同時生效。

## 3. 處理策略設定

- [x] 3.1 讓 `get_settings` 回報新的處理策略設定：缺鍵時回報 `auto`，值不在 `auto`／`original`／`compat` 之內時保留 `auto` 且不回寫資料庫，並且 `auto` 不得被改寫成偵測結果而必須原樣儲存。交付 `Download Video Handling Setting Key` 與 `Default Settings Values`，並落實 design 決策「處理策略是三值設定，`auto` 於每次下載時解析」。實作位置為 `src-tauri/src/commands/settings.rs` 的 `Settings` 結構、其 `Default` 實作與 `merge` 邏輯。驗證：新增單元測試涵蓋三個合法值各自被接受、一個不合法值落回 `auto`、缺鍵時為 `auto`。
- [ ] 3.2 讓設定頁可切換三種策略，且在 `auto` 時顯示當下解析到的結果，並說明保留原檔的取捨（畫質更好、檔案更小、幾乎瞬間完成，但可能在其他裝置無法播放）。交付 `Resolved Auto Policy Is Shown`。實作位置為 `src/lib/stores/settings.svelte.ts` 的 `Settings` 介面與 `src/routes/settings/+page.svelte`；需要一個讓前端取得偵測結果的途徑。驗證：手動開啟設定頁，確認三個選項可切換、`auto` 時顯示解析結果、且取捨說明可見；切換到 `compat` 與 `original` 後說明文字隨之變化。

## 4. 後處理決策改由白名單與策略共同決定

- [x] 4.1 把後處理決策改為純函式，同時接受探測結果、處理策略與平台能力答案三個輸入，回傳既有的 remux 或重新編碼計畫；可 remux 的視訊 codec 限定為 H.264 與 AV1，其餘一律重新編碼，音訊與尺寸條件沿用既有規則。交付 `Post-Download Container Optimization`，並落實 design 決策「remux 適用範圍只擴充到 AV1，且沿用既有的音訊與尺寸條件」。平台查詢的結果須由呼叫端傳入而非在函式內查詢，以保持可測試。驗證：新增單元測試涵蓋 video-download-engine spec 的「decision table」全部 12 列。
- [ ] 4.2 把 4.1 接入下載流程：讀出處理策略設定、取得平台對探測到的視訊 codec 的能力答案，交給決策函式，並讓進度事件的狀態文字反映實際走的分支。讀取設定失敗時退回 `auto` 而非讓下載失敗。驗證：手動在偵測到支援 AV1 的機器上以 `auto` 下載一支 Facebook Reel，確認狀態文字為容器最佳化、輸出的視訊 codec 為 `av1`、檔案大小接近下載到的原檔、後處理耗時遠短於重新編碼。
- [ ] 4.3 確認 `compat` 策略下的行為與本變更前一致（除音訊那一項）。驗證：手動以 `compat` 下載同一支 Reel，確認輸出視訊 codec 為 `h264`，且以 ffprobe 比對音訊位元率與下載到的原檔相同（證明音訊未被重新編碼）。

## 5. 文件

- [x] 5.1 [P] 在 `docs/windows-verification.md` 的待驗清單加入 Windows 端的平台偵測項目：在有／無 AV1 解碼支援的 Windows 環境各驗一次 `auto` 的解析結果與實際輸出 codec。驗證：內容審閱，確認兩種環境的預期結果都寫明。
