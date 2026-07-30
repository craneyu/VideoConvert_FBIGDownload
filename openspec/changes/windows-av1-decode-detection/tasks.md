## 1. 依賴與平台分支骨架

- [x] 1.1 [P] 依「windows crate 設為 target-gated 直接依賴」的決策，讓 Windows target 能呼叫 Media Foundation API：在 src-tauri/Cargo.toml 以 Windows 條件加入 windows crate（版本對齊 Cargo.lock 現有的 0.61.3，只啟用 Media Foundation 所需 feature），並加註解說明這是既有傳遞依賴的提升、不是新的體積負擔。驗證：在 Windows 上 cargo check --all-targets 通過，且 Cargo.lock 沒有出現第二份 windows 主版本。
- [x] 1.2 [P] 把 codec_support.rs 的非 macOS 平台實作拆成 Windows 與其他平台兩條，行為暫時不變（兩者都回 unknown），Linux 的答案逐字維持現況。驗證：cargo test --lib 在 Windows 與 macOS 都全綠，既有的 memoisation 與 panic 轉 unknown 測試不需修改即通過。

## 2. Windows 解碼能力查詢（測試先行）

- [x] 2.1 先寫失敗測試涵蓋 Windows Decode Capability Query 的四個情境：找到解碼器回 supported、列舉成功但沒找到回 unsupported、列舉本身失敗回 unknown、未對應的編碼名回 unknown。測試點放在可注入列舉結果的純函式上（比照既有的 support_from_lookup），不依賴執行機器實際裝了什麼解碼器。驗證：四個測試在實作前失敗、實作後通過，且在 macOS 上不編譯。
- [x] 2.2 依「用 MFTEnumEx 只取解碼器數量，不實例化解碼器」實作查詢：把 ffprobe 的編碼名不分大小寫對應到 Media Foundation 視訊子型別（至少 av1 與 h264），以視訊解碼器類別列舉，數量大於零即 supported，並釋放回傳的 activate 指標與陣列。驗證：2.1 的四個測試通過；在本機執行 decodable_video_codecs 回傳包含 av1。
- [x] 2.3 鎖定「軟體解碼也算 supported，不過濾硬體旗標」這個決策：列舉不得帶硬體限制旗標，並加一個測試斷言查詢參數未設硬體限制，使日後有人改成只認硬體解碼時測試會失敗。驗證：該測試通過，且在沒有 AV1 硬體解碼路徑的本機上 av1 仍回 supported。
- [x] 2.4 讓 Per-Platform Detection Strictness 成立：把既有的 non_macos_platforms_answer_unknown_for_every_codec 測試改為只涵蓋 Linux，並新增 Windows 專屬測試斷言 av1 的答案是 supported 或 unsupported 而非 unknown。驗證：cargo test --lib 在 Windows 全綠，且該 Linux 測試在 Windows 上不再編譯。

## 3. 跨平台答案一致性

- [ ] 3.1 確認「三態語意寫進規格：允許各平台不同保守程度」所描述的行為在三個平台都成立，即 Three-State Codec Decode Capability 的平台對照表為真：macOS 仍以硬體查詢回 supported 或 unsupported、Windows 可回 supported 或 unsupported、Linux 對每個編碼都回 unknown。驗證：三個平台各自的 cargo test --lib 全綠，且只有 supported 會讓下載路徑選擇保留原始串流（既有的 plan_post_processing 測試涵蓋此點，不需新增）。

## 4. 兩種實機環境驗證

- [ ] 4.1 在有 AV1 解碼能力的 Windows 上驗證正向路徑：設定頁「自動判斷」的說明文字從「未能確認本機可解碼 AV1 → 重新編碼為 H.264」變為「本機可解碼 AV1 → 保留原始畫質」，且下載一支 AV1 來源的 Facebook Reel 後，後處理狀態文字為容器最佳化、輸出視訊編碼為 av1、檔案大小接近原檔。驗證：目視設定頁文字，並以 ffprobe 確認輸出編碼與大小。
- [ ] 4.2 依「用移除 AV1 Video Extension 取得無支援環境來驗證負向路徑」驗證反向行為：暫時移除 Microsoft.AV1VideoExtension、重啟 App（能力查詢每個行程只做一次，不重啟不會反映），確認自動判斷回到重新編碼、輸出編碼為 h264，驗完從 Store 裝回。驗證：以 ffprobe 確認輸出為 h264，且設定頁文字回到未能確認的版本。
- [x] 4.3 [P] 讓 Windows 的單元測試在 CI 上持續被執行：在 .github/workflows/release.yml 的 windows-latest 工作加入 cargo test --lib，並確認該 runner 屬於無 AV1 解碼能力環境（av1 回 unsupported），因此天然覆蓋負向路徑。驗證：CI 該工作通過，log 顯示測試數與本機一致。

## 5. 文件與發佈說明

- [x] 5.1 [P] 讓發佈說明不再與實際行為矛盾：更正 .github/workflows/release.yml 中「Windows 與 Linux 一律視為無法確認」的敘述，並寫出使用者可見的預設輸出編碼變更（有解碼能力的 Windows 在自動判斷下會保留 AV1）以及改用相容優先覆寫的方式。驗證：內容審閱該段文字，確認三個平台的敘述與實作一致。
- [ ] 5.2 [P] 重寫 docs/windows-verification.md 的 J 節，使其從「驗證現況是否如預期（尚未實作）」改為「驗證自動判斷是否正確」，並保留既有的實測數據作為對照基準。需等目前正在審查的文件 PR 合併後再進行，避免同檔衝突。驗證：內容審閱，確認 J 節不再宣稱 Windows 尚未實作，且步驟涵蓋有／無解碼能力兩種環境。
