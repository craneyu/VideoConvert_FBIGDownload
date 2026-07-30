## Context

`platform_support` 目前只有兩個分支：macOS 用 VideoToolbox 查詢硬體解碼能力，其餘平台一律回 `Unknown`。因為只有 `Supported` 才算許可，Windows 的「自動判斷」政策永遠落到重新編碼，等於保留原始畫質這個功能在 Windows 上沒有生效。

實測（Windows 11 25H2，build 26200）一支真實 Facebook Reel（AV1 Main + HE-AAC，1080x1920，9.38 秒，2,068,245 bytes）：重新編碼輸出 4,115,940 bytes、耗時 5.41 秒；保留原始串流輸出 2,068,245 bytes、耗時 0.07 秒。也就是目前 Windows 使用者付出的代價是檔案大 99%、後處理慢 77 倍。

當初延後的三個理由都已消失：

- 建置：Windows 端 cargo test 與 cargo check 皆通過（需先初始化 MSVC 環境，見 docs/windows-verification.md 的 A-0）。
- 驗證環境：本機已安裝 Microsoft.AV1VideoExtension，屬「有解碼能力」環境；GitHub 的 windows-latest runner 幾乎確定沒有，屬「無解碼能力」環境。兩種都有。
- 依賴：windows crate 0.61.3 已透過 tauri 存在於 src-tauri/Cargo.lock，提升為直接依賴不是新的下載或建置負擔。

一個必須先解決的語意衝突：規格的要求寫「平台能不能解碼」，但 macOS 的實作用 `VTIsHardwareDecodeSupported`，只回答硬體解碼。本機正是分歧點——沒有 AV1 硬體解碼路徑（ffmpeg 的 d3d11va 明確回報不支援），卻能透過擴充軟體解碼。同一台機器在兩種語意下答案相反。

## Goals / Non-Goals

**Goals:**

- 讓 Windows 回答真實的解碼能力，而不是一律「未知」。
- 保留既有的三態契約與「只有 supported 才是許可」的不對稱設計。
- 把跨平台的保守程度差異寫進規格，讓 macOS 的刻意低報從隱性不一致變成明文允許。
- 讓負向路徑（無解碼能力）也能被實際驗證，而不只是推論。

**Non-Goals:**

- 不做 Linux 的偵測。系統層沒有可代表「使用者的播放器能不能解」的東西，維持 `Unknown`。
- 不改 macOS 的語意。它繼續以硬體解碼查詢刻意低報，這是安全方向。
- 不判斷「解得夠不夠快」。Media Foundation 只回答有沒有解碼器。
- 不用 Windows 組建號或 AV1 擴充是否安裝來做推測式判斷——那是猜測，不是查詢。
- 不改動 remux 的資格規則（音訊須為 AAC、長寬須為偶數）與檔名長度預算。
- 不改前端。設定頁文字由既有查詢驅動，會自動反映新答案。

## Decisions

### 軟體解碼也算 supported，不過濾硬體旗標

Windows 的列舉不加硬體限制，任何可用的 AV1 解碼器都算 `Supported`。

理由是模組自己界定的問題範圍：它回答的是「這台機器能不能解這個編碼」，而且已明講這不是「檔案傳到別處也能播」的保證。軟體解碼確實能播，所以回 `Supported` 沒有違反「絕不產出播不了的檔案」的不對稱設計。

考慮過的替代方案：只認硬體解碼（用硬體列舉旗標），與 macOS 語意對齊。否決原因是收益幾乎歸零——開發與驗證用的這台機器沒有 AV1 硬體解碼，會回 `Unsupported`，實作完看不到任何行為差異，而放棄的收益已量測為檔案大 99%、慢 77 倍。語意一致本身不是使用者價值。

代價（接受並記錄於風險）：低階機器可能保留一支播放時會卡頓的高解析度 AV1。

### 用 MFTEnumEx 只取解碼器數量，不實例化解碼器

判斷只需要「找不找得到解碼器」，因此呼叫 `MFTEnumEx` 查詢視訊解碼器類別、輸入型別為目標編碼，取回傳數量：大於零為 `Supported`，等於零為 `Unsupported`，呼叫失敗為 `Unknown`。回傳的 activate 指標與陣列必須釋放，但不需要實例化任何解碼器，也不需要讀取解碼器名稱。

這讓 COM 接觸面遠小於「寫一整套 COM 程式碼」——原本延後理由中對 COM 複雜度的顧慮，主要來自假設要建立並操作解碼器物件。

考慮過的替代方案：用 shell 的縮圖提供者實際解一張畫格（本次驗證就是這樣證明本機可解 AV1 的）。否決原因是它需要一個真實檔案，而能力查詢發生在還沒有輸出檔的時候，且成本遠高於列舉。

**實作前先以 spike 驗證的兩件事**（結果比原本假設的更好，兩者都已反映在實作中）：

1. **桌面行程看得到 appx 提供的解碼器。** 最大的疑慮是本機的 AV1 解碼器來自 Microsoft Store 擴充（av1decodermft_store.dll，appx 註冊，不在傳統登錄路徑下），非封裝的桌面行程可能列舉不到而得到偽陰性。實測 MFTEnumEx 以 MFT_ENUM_FLAG_SYNCMFT 對 AV1 回 count=1，確認看得到。
2. **不需要任何 COM 初始化。** MFTEnumEx 是登錄檔支撐的查詢，實測在既沒有呼叫 CoInitializeEx、也沒有呼叫 MFStartup 的情況下回 S_OK 並得到正確數量；未知的編碼子型別回 count=0（而非錯誤）。因此實作不引入 MFStartup 的行程級副作用，也沒有 apartment 模型的問題。

同一次 spike 也顯示 MFT_ENUM_FLAG_HARDWARE 單獨使用時對 AV1 與 H264 都回 count=0，這佐證了不過濾硬體旗標的決策：硬體 MFT 的列舉在這個呼叫情境下並不可靠，用它當唯一依據會把能播的機器判成不能播。

### windows crate 設為 target-gated 直接依賴

在 src-tauri/Cargo.toml 以 Windows target 條件加入 windows crate，只啟用 Media Foundation 相關 feature。版本對齊 Cargo.lock 現有的 0.61.3，避免拉進第二份主版本。

依照專案既有慣例（tokio 那段註解記錄了同樣的論證）：這是把已經編譯進二進位檔的傳遞依賴提升為直接依賴，因此註解要說明清楚它不是新的體積負擔，以免日後有人以為可以移除。

### 三態語意寫進規格：允許各平台不同保守程度

規格新增一條要求，明訂各平台允許不同的保守程度，但任何平台都不得把「播不了」報成 supported；並更新平台對照表，Windows 從「只能回 unknown」改為以 Media Foundation 列舉回 supported 或 unsupported，同時標註 macOS 的硬體查詢是刻意低報。

沒有這一條，Windows 比 macOS 寬鬆會看起來像 bug，而不是決策。

### 用移除 AV1 Video Extension 取得無支援環境來驗證負向路徑

規格要求在有／無解碼能力的兩種環境各驗一次。不需要第二台機器：在驗證機上暫時移除 Microsoft.AV1VideoExtension 即可得到無支援狀態，驗完再從 Store 裝回。CI 的 windows-latest runner 則是天然的無支援環境。

因為能力查詢是每個行程只做一次並快取，切換擴充狀態後必須重啟 App 才會反映——這是既有的規格行為，驗證步驟必須包含重啟。

## Implementation Contract

**行為**：在能解 AV1 的 Windows 機器上，「自動判斷」政策改為保留下載到的 AV1 原始串流（走容器最佳化），輸出視訊編碼為 av1、大小接近原檔、後處理接近瞬間完成。無解碼能力或查詢失敗的機器行為完全不變，仍重新編碼為 H.264。「保留原檔」與「相容優先」兩個政策的行為不受影響。

**介面與資料形狀**：不新增 IPC command，不改任何函式簽章。既有的 seam `platform::support` 仍是這條路徑上唯一的 adapter，回傳既有的三態 `DecodeSupport`。`decodable_video_codecs` 的回傳形狀不變（字串陣列），只是在 Windows 上可能開始包含 av1 或 h264。快取與 panic 轉 `Unknown` 由既有的 `memoised` 承接，不得另建快取。

**編碼名對應**：查詢需把 ffprobe 的編碼名對應到 Media Foundation 的視訊子型別，至少涵蓋 av1 與 h264，比較時不分大小寫。沒有對應的編碼名回 `Unknown`，與 macOS 在找不到四字元代碼時的行為一致。

**失敗模式**：列舉回傳失敗或呼叫過程 panic，一律回 `Unknown`，不得讓下載失敗，也不得彈出任何 UI。失敗是靜默的，因為 `Unknown` 的後果只是走原本就會走的重新編碼路徑。（原先也列了「COM 初始化失敗」，spike 證明這條路徑不需要任何 COM 初始化，因此不存在這個失敗模式。）

**驗收標準**：

- 在 Windows 上 cargo test --lib 全綠，並新增 Windows 專屬測試涵蓋：列舉成功且找到解碼器回 supported、列舉失敗回 unknown、未對應的編碼名回 unknown。
- 在有 AV1 擴充的機器上，設定頁「自動判斷」的說明文字從「未能確認本機可解碼 AV1 → 重新編碼為 H.264」變為「本機可解碼 AV1 → 保留原始畫質」。
- 下載一支 AV1 來源的 Facebook Reel，自動判斷下的輸出視訊編碼為 av1 且大小接近原檔（取代目前的 h264 與大 99%）。
- 移除 AV1 擴充並重啟後，同一支影片的自動判斷回到輸出 h264。
- CI 的 windows-latest 執行單元測試通過。

**範圍邊界**：

- 在範圍內：src-tauri/src/commands/codec_support.rs 的非 macOS 分支拆分與 Windows 實作、src-tauri/Cargo.toml 的依賴、platform-codec-capability 的規格 delta、.github/workflows/release.yml 的發佈說明文字與 Windows 單元測試、docs/windows-verification.md 的 J 節重寫。
- 在範圍外：Linux 偵測、macOS 語意調整、播放流暢度判斷、前端改動、下載管線的其他決策規則、以及該驗證文件其他章節的內容。

## Risks / Trade-offs

- [軟體解碼但機器算力不足，保留下來的高解析度 AV1 播放卡頓] → 政策可由使用者覆寫為「相容優先」，且設定頁已有「偵測只反映這台機器」的警語。本次不做效能判斷，因為 Media Foundation 無法回答這個問題，而猜測門檻比誠實回答更糟。
- [Media Foundation 回報有解碼器，實際播放仍失敗（偽陽性）] → 這是唯一會產出播不了檔案的情境，因此驗收標準要求在兩種環境各驗一次實際輸出。若實測出現偽陽性，退路是改為只認硬體解碼列舉，不需要改動任何介面。
- [COM 初始化與 Tauri 主執行緒的 apartment 模型衝突] → **已由 spike 排除**：MFTEnumEx 不需要 CoInitializeEx 或 MFStartup，實測在完全未初始化 COM 的行程中回 S_OK。查詢仍包在既有的 `memoised` 內，任何失敗或 panic 都收斂為 `Unknown`。
- [使用者可見的預設輸出編碼改變] → 發佈說明必須明確寫出這個變更與覆寫方式，否則習慣拿到 H.264 的使用者會以為是 bug。
- [docs/windows-verification.md 目前有另一個進行中的 PR 也在改同一個檔案] → J 節重寫等該 PR 合併後再進行，避免衝突。
