## Why

VidBridge 0.1.3 已對外發佈，但三類缺陷同時存在，且都落在使用者的主要操作路徑上。

**自動更新從未對任何人運作過。** updater 設定的 pubkey 欄位放的是 passphrase 加密的 minisign **私鑰**，不是公鑰 —— base64 解碼後的註解字串為 "untrusted comment: rsign encrypted secret key"，且內容含 KDF salt 與 opsLimit/memLimit 欄位，這些只存在於私鑰檔案（公鑰為單行約 56 字元且無 KDF 欄位）。同時 updater endpoint 指向的更新 manifest 從未進入版控，因此更新檢查連抓取 manifest 就會失敗。前端的更新檢查把所有錯誤吞進空的 catch，導致這個問題兩個月來沒有任何徵兆。私鑰進入公開版控屬於「應盡快輪替、可離線暴力破解」等級的暴露 —— passphrase 存放於 GitHub Secrets，尚未構成已被攻破。

**六個使用者可見的 bug。** 頁面往返後拖放檔案會產生重複轉檔任務；下載失敗的紀錄在歷史頁永遠停留在 downloading；歷史日期在本地時間 00:00–08:00 建立的紀錄會少一天；「開啟檔案位置」在 Windows 與 Linux 是無聲空操作，但發佈流程確實在建置 Windows 安裝檔；H.265 轉檔輸出因缺少容器 tag 而無法在 QuickTime 播放；下載失敗時使用者只看到硬編碼字串，真正原因（私人影片、需登入、地區限制）全部被丟棄，且未被讀取的 stderr 管線在輸出量大時會讓子行程阻塞而卡死下載。

**每次下載都無條件重新編碼。** 下載完成後一律以 libx264 重編一次，但 Facebook 與 Instagram 的來源檔絕大多數已是 H.264 + AAC 的 MP4。代價是下載總時間被拉長數倍（進度條在 90–95% 停滯即為此段），以及不可逆的二次壓縮畫質損失。

**為什麼是現在**：正因為自動更新對所有使用者皆未成功運作過，沒有任何既存安裝基數依賴舊金鑰，現在輪替不會孤立任何使用者，代價最低。且這批修復的驗證需要一次真實 release 才算完成，把三組併為同一批可共用同一次發佈驗證。

## What Changes

分為三組。A 組含一項對使用者可見的破壞性影響。

### A. 自動更新與發佈管線

- **BREAKING**：輪替簽章金鑰對，updater 設定改填真正的公鑰，舊金鑰停用。既有 0.1.3 安裝無法透過自動更新取得新版，使用者需手動下載一次修復版，之後自動更新才會接上。因既有自動更新本來就不能運作，此破壞性影響不減損任何現有功能。
- 發佈可實際取得的更新 manifest，並使其檔名與 updater endpoint 一致。
- 前端更新檢查失敗不再靜默：改為寫入 log，讓下次失效可被發現。
- release workflow 觸發條件由「推送 main」改為「推送版本 tag」與手動觸發，避免每次推送都重跑三平台建置並把產物重複上傳到既有 release。
- 新增金鑰輪替與 GitHub Secrets 設定步驟文件，讓此流程可被重複執行。

### B. 使用者可見 bug

- 事件監聽（下載進度、轉檔進度、檔案拖放）改為可正確清理，修掉頁面往返後拖放產生重複任務。
- 下載失敗時把 `download_history` 對應紀錄的狀態寫回 failed。
- 下載紀錄時間戳改存本地時間。以新增 migration 處理，不修改既有 migration（既有安裝已執行過，修改不會重跑），並校正既有資料。
- 開啟檔案位置支援 Windows 與 Linux，不再只在 macOS 生效。
- H.265 輸出至 MP4 容器時加上 `hvc1` 相容 tag。
- 讀取 yt-dlp 的 stderr 並在失敗時回傳實際錯誤內容，同時消除管線塞滿導致的阻塞風險。

### C. 下載改條件式 remux

- 下載完成後先以 ffprobe 判斷影音 codec 與畫面尺寸；已相容者僅做容器 remux（`-c copy` 搭配 faststart），不相容者才重新編碼。決策結果需可觀測，以便確認走到哪條分支。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-auto-update`: 更新驗簽須使用真正的公鑰、更新 manifest 須可取得、更新檢查失敗須可觀測
- `video-download-engine`: 新增下載後條件式容器 remux 決策；下載失敗須回傳來源工具的實際錯誤且不得因未讀取管線而阻塞
- `download-history-storage`: 下載失敗須寫回紀錄狀態；紀錄時間戳須為本地時間
- `download-ui-integration`: 開啟檔案位置須在 macOS 以外平台同樣生效
- `video-transcoding-engine`: H.265 輸出至 MP4 須帶容器相容 tag
- `transcoding-ui-integration`: 拖放事件監聽不得因頁面往返而重複註冊

## Impact

- Affected specs: `app-auto-update`、`video-download-engine`、`download-history-storage`、`download-ui-integration`、`video-transcoding-engine`、`transcoding-ui-integration`
- Affected code:
  - Modified:
    - `src-tauri/tauri.conf.json`
    - `src-tauri/src/lib.rs`
    - `src-tauri/src/commands/download.rs`
    - `src-tauri/src/commands/transcode.rs`
    - `src/routes/+page.svelte`
    - `.github/workflows/release.yml`
  - New:
    - `docs/updater-key-rotation.md`
  - Removed: (none)
- 需人工執行且無法由實作代理完成的前置動作：產生新簽章金鑰對、將私鑰與 passphrase 寫入 GitHub Secrets。此步驟阻擋 A 組後續任務的驗證。
- 外部依賴：ffprobe 由條件式 remux 的判斷步驟使用，已列於既有依賴檢查清單，不新增依賴。
