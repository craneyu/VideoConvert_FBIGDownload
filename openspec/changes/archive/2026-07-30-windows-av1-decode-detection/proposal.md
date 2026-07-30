## Why

Windows 上的 AV1 解碼能力查詢目前一律回「未知」，因此「自動判斷」政策永遠保守地走重新編碼，等於此功能在 Windows 上沒有生效。實測代價比原先評估的更大：一支真實 Facebook Reel（AV1 + HE-AAC，1080x1920）重編後檔案大 99%、耗時 5.41 秒；保留原始串流只需 0.07 秒且位元組數與原檔相同。

當初延後的三個理由現在都不成立：Windows 建置環境已驗證可用（cargo test 全綠）、CI 的 release matrix 早已包含 windows-latest、而 windows crate 0.61.3 已透過 tauri 存在於 Cargo.lock，不是新的下載或建置負擔。

## What Changes

- Windows 改用 Media Foundation 的解碼器列舉（MFTEnumEx）回答 AV1 解碼能力：找到解碼器回 supported、列舉成功但沒有回 unsupported、列舉本身失敗回 unknown。
- **不過濾硬體解碼旗標** —— 軟體解碼也算 supported。判斷的問題是「這台機器能不能解這個編碼」，而軟體解碼確實能播。
- 規格層明文化跨平台語意：各平台允許不同的保守程度（macOS 以硬體解碼查詢刻意低報），但任何平台都不得把「播不了」報成 supported。
- windows crate 成為 target-gated 直接依賴，只啟用 Media Foundation 所需 feature。
- Linux 行為不變，仍然只能回 unknown。
- **使用者可見的預設行為變更**：有 AV1 解碼能力的 Windows 機器在「自動判斷」下會開始保留 AV1 原始串流，而非輸出 H.264。這不是規格破壞（只有 supported 才是許可，契約不變），但輸出編碼確實會變，使用者可用「相容優先」覆寫。
- 發佈說明與驗證文件中「Windows 一律視為無法確認」的敘述需同步更正，否則會與實際行為不符。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `platform-codec-capability`: Windows 從「只能回 unknown」變成可回 supported 或 unsupported；並新增一條要求，明訂各平台允許不同保守程度但不得把播不了報成 supported。

## Impact

- Affected specs: platform-codec-capability
- Affected code:
  - New: (none)
  - Modified:
    - src-tauri/src/commands/codec_support.rs
    - src-tauri/Cargo.toml
    - .github/workflows/release.yml
    - docs/windows-verification.md
  - Removed: (none)
- Affected dependencies: windows crate 由傳遞依賴提升為 Windows target 的直接依賴。
- 不影響前端：設定頁的說明文字由既有的 decodable_video_codecs 查詢驅動，會自動反映新答案，無需改動。
