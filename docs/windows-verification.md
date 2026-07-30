# Windows 驗證清單

本文原是為驗證 `fix/tray-settings-navigation` 分支（PR #6）而寫，該 PR 已合併；現在是拿來驗當前的 `main`（最近一輪驗證跑在 **v0.1.6**）。各節記錄的實測結果都會註明所在版本。

**為什麼需要這份清單**：所有 `#[cfg(target_os = "windows")]` 的程式碼在 macOS 上**從未被編譯過**，因此 macOS 端的 `cargo check` 與 `cargo test` 全綠**不代表** Windows 能建置成功。下方 A 組就是為此而設。

已在 macOS 驗證完成的項目（不需在 Windows 重驗邏輯，但仍需確認平台行為）：狀態列 Settings 導航、選單中文化、設定頁單畫面、下載歷史資料夾圖示、檔名長度限制、下載失敗狀態寫回、時間戳本地化、條件式 remux、H.265 的 hvc1 tag。

---

## 前置環境

| 項目 | 說明 |
| --- | --- |
| Rust | `rustup default stable-x86_64-pc-windows-msvc` |
| C++ 建置工具 | Visual Studio Build Tools，需含 **Desktop development with C++**（`ring` 等依賴需要 C 編譯器）。裝了不等於能建置 —— 見 A 節開頭的 vcvars 說明 |
| WebView2 | Windows 11 內建；Windows 10 需自行安裝 Evergreen Runtime |
| Node.js | LTS |
| 外部工具 | `ffmpeg`、`ffprobe`、`yt-dlp`（App 會自行偵測並可代為安裝，見 E 組） |

```powershell
git fetch origin
git checkout main
git pull --ff-only
npm install
```

---

## A. 先確認能編譯（最高優先，且最省時間）

### A-0. 先初始化 MSVC 環境，否則建置失敗與程式碼無關

**在裸 PowerShell 直接跑 `cargo` 可能失敗，而且錯誤看起來像程式問題。** 已在 Windows 11 實測遇到：

```
cc-rs: windows.h(171): fatal error C1083: Cannot open include file: 'excpt.h'
LINK : fatal error LNK1104: cannot open file 'msvcrt.lib'
```

兩者都是 `INCLUDE` / `LIB` 沒設定的症狀（cargo 的 build script 輸出會顯示 `INCLUDE = None`），不是 Windows 專屬區塊有錯。

**成因**：機器上裝了多份 Visual Studio 時，rustc 可能挑到其中一份的 `cl.exe`，卻沒有對應的 Windows SDK 路徑。實測機上有 Enterprise 2026、Enterprise 2022、Build Tools 2022 三份，而兩份 **Enterprise 的 `VC\Auxiliary\Build` 只有 `vcvars64.bat` 卻缺 `vcvarsall.bat`** —— `vcvars64.bat` 內部要呼叫 `vcvarsall.bat`，所以連它自己都跑不起來：

```
'"...\VC\Auxiliary\Build\vcvarsall.bat"' is not recognized as an internal or external command
```

只有 Build Tools 2022 帶完整的 vcvars 腳本組。

**做法**：把建置指令包在 Build Tools 的 `vcvars64.bat` 裡，不要假設裸環境可用。

```powershell
cmd /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1 && cargo test --lib'
```

先確認哪些 VS 有完整腳本（挑有 `vcvarsall.bat` 的那份）：

```powershell
& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe" -products * -property installationPath
Get-ChildItem "<上面某個路徑>\VC\Auxiliary\Build" -Filter "vcvars*"
```

或改用「x64 Native Tools Command Prompt for VS」開一個已初始化的終端機再跑後續指令。**建置一失敗就先確認這件事**，再去懷疑程式碼。

### A-1. 編譯與單元測試

**先跑 `cargo check` 再跑完整建置。** Windows 專屬區塊若有型別錯誤，`cargo check` 幾十秒就會報出來，不必等完整建置。

```powershell
cd src-tauri
cargo check
cargo test --lib
```

從未在任何平台編譯過的程式碼：

| 位置 | 內容 |
| --- | --- |
| `src-tauri/src/commands/download.rs` | `reveal_in_file_manager` 的 Windows 分支（**本次新增**） |
| `src-tauri/src/commands/utils.rs` | `hidden_cmd` 的 `CommandExt` / `creation_flags` |
| `src-tauri/src/commands/utils.rs` | `find_tool_path` 的 Windows 路徑搜尋區塊 |
| `src-tauri/src/commands/utils.rs` | `install_tool_windows` |

**預期**：`cargo check` 無錯誤；`cargo test --lib` 全綠（測試本身與平台無關，數字應與 macOS 一致 —— 會隨後續 change 增加，v0.1.6 當時為 **104 passed**）。

接著：

```powershell
cd ..
npm run tauri dev
```

---

## B. 「開啟檔案夾」— 已在 Windows 11 實測，確認為 bug 並已修復

原始程式碼把 `/select,C:\路徑\檔名.mp4` 當成**單一參數**傳給 explorer。Rust 的 `Command` 在參數含空白時會把**整段**包上引號，變成 `"/select,C:\有 空白\檔名.mp4"`，explorer 無法解析。

**實測結果**（Windows 11，以相同 `Command` 呼叫方式重現，並用 Shell COM 讀出實際開啟位置與選取項目）：

| 版本 | 路徑 | 實際開啟位置 | 選中檔案 |
| --- | --- | --- | --- |
| 修復前 | 含空白 | `C:\Users\<你>\OneDrive\文件` ❌ | 0 個 ❌ |
| 修復前 | 不含空白 | 正確資料夾 ✅ | 1 個 ✅ |
| 修復後 | 含空白 | 正確資料夾 ✅ | 1 個 ✅ |
| 修復後 | 不含空白 | 正確資料夾 ✅ | 1 個 ✅ |

修法是改用 `raw_arg`，讓引號只包住路徑（`/select,"C:\...\clip.mp4"`）。參數組裝已抽成 `explorer_select_arg`，並**刻意不加 `#[cfg(target_os = "windows")]`**，這樣三個對應測試在 macOS 上也會執行 —— 這個 bug 正是因為該分支在開發機上從未被執行、甚至從未被編譯才流出。

**仍建議在實機複驗一次**（走完整 UI 路徑，而非只驗參數字串）：

| 測試 | 步驟 | 預期 |
| --- | --- | --- |
| B-1 路徑**不含**空白 | 下載一支影片到 `C:\Users\<你>\Downloads\` → 下載歷史 → 點資料夾圖示 | 檔案總管開啟且**該檔案被選中** |
| B-2 路徑**含**空白 | 設定頁把下載路徑改到含空白的資料夾（例如 `C:\Users\<你>\My Videos`）→ 下載 → 點資料夾圖示 | 同上 |

---

## C. 檔名長度 — macOS 與 Windows 的限制**不同**，這是設計上的已知落差

`bound_filename` 的預算是 **200 bytes**，這個數字是依 macOS/APFS 的「255 **bytes** 單一元件上限」訂的。Windows 不一樣：

| | macOS / APFS | Windows / NTFS |
| --- | --- | --- |
| 單一元件上限 | 255 **bytes** | 255 **字元**（UTF-16） |
| 完整路徑上限 | 1024 bytes | **260 字元**（MAX_PATH，未啟用長路徑時） |

推算：

- **中日文標題**：200 bytes ≈ 66 字元 → 元件與完整路徑都寬鬆，安全
- **純英文標題**：200 bytes = 200 字元。加上路徑前綴 `C:\Users\<你>\Downloads\VidBridge\Facebook\`（約 45 字元）= 約 245 字元，**低於 260 但很接近**
- **英文標題 + 較深的下載路徑**：前綴若達 75 字元（例如 `C:\Users\LongUserName\Documents\My Videos\Downloads\VidBridge\Facebook\`），200 + 75 = **275 字元 → 超過 MAX_PATH**

| 測試 | 步驟 | 預期 |
| --- | --- | --- |
| C-1 中日文長標題 | 下載一支貼文說明很長的日文/中文 FB reel | 成功，檔名被截短且無亂碼 |
| C-2 英文長標題 + 深路徑 | 下載路徑設為層數較深、名稱較長的資料夾，下載一支**英文**長標題影片 | 見下方實測 —— **上面的推算沒有成立** |

### C-2 實測結果：沒有失敗，因為寫檔的是 ffmpeg

在 Windows 11 25H2 實測（`LongPathsEnabled = 0`，所以 260 字元上限確實生效），固定 200 字元檔名並逐步加長路徑前綴：

| 前綴長度 | 完整路徑總長 | 結果 |
| --- | --- | --- |
| 41 字元（`C:\Users\<你>\Videos\VidBridge\Facebook\`） | 241 | 成功 |
| 59 / 60 / 64 / 71 字元 | 259 / 260 / 264 / 271 | **全部成功** |
| 100 / 150 / 200 字元 | 300 / 350 / 400 | **全部成功**，輸出位元組數與來源一致 |

**為什麼推算沒成立**：最終檔案的寫入者是 **ffmpeg**，不是 Rust 那一側。實測的 ffmpeg 8.1.1（gyan.dev build）會自行處理超長路徑；同一個 271 字元路徑用傳統 Win32 ANSI 路徑的 `cmd copy` 寫則如預期失敗（`The system cannot find the path specified.`），可見系統層的 260 限制是真的存在，只是 ffmpeg 繞過了它。

所以**不需要**改 `bound_filename` 的預算，上面「275 字元 → 超過 MAX_PATH」那條推算對本專案的實際寫入路徑不適用。

**殘餘風險（尚未驗，因為需要開視窗）**：檔案建得出來不代表別的程式讀得到。B 節的「開啟檔案夾」走的是 `explorer /select,<完整路徑>`，而 Explorer 對超過 260 字元的路徑向來不可靠；同理，使用者拿其他播放器開這個檔也可能失敗。若要驗，請在下載路徑很深的情況下實際點一次資料夾圖示。

若日後真的遇到路徑過長錯誤，修法是把預算改成同時考慮「完整路徑字元數」而非只看檔名位元組數；請回報後再處理，不要自行加大預算。

---

## D. 主控台視窗不應閃現

`hidden_cmd` 對每個子行程加上 `CREATE_NO_WINDOW`，這是它存在的唯一理由。

| 測試 | 預期 |
| --- | --- |
| 啟動 App、下載一支影片、轉檔一支影片 | 全程**不應**看到黑色主控台視窗閃現（yt-dlp、ffmpeg、ffprobe、winget 都會被啟動） |

---

## E. 外部工具偵測與安裝

`find_tool_path` 在 Windows 會搜尋 choco、winget（含 `Gyan.FFmpeg` 的版本無關路徑）、scoop、pip 等位置。

| 測試 | 步驟 | 預期 |
| --- | --- | --- |
| E-1 已安裝時 | 三個工具都裝好後啟動 App | 不彈出安裝詢問，下載/轉檔正常 |
| E-2 缺少時 | 暫時把某個工具移出 PATH 與上述目錄後啟動 | 彈出詢問是否自動安裝；同意後透過 winget 或 choco 安裝成功 |

**已知待改善（非本次範圍）**：App 每次啟動都會在背景執行一次 yt-dlp 升級。在 Windows 上這會呼叫 winget，可能明顯拖慢啟動。這是評估報告列出的 T2 項目，尚未修。若你覺得啟動變慢，原因在此。

---

## F. 狀態列（通知區域）

| 測試 | 預期 |
| --- | --- |
| F-1 選單語言 | 右鍵狀態列圖示 → 顯示 **顯示視窗 / 軟體設定 / 結束程式**（中文） |
| F-2 Settings 導航 | 視窗**可見**時點「軟體設定」→ 切換到設定畫面（這是本次修的 bug；修復前毫無反應） |
| F-3 視窗隱藏時 | 關閉視窗（會隱藏而非結束）→ 點「軟體設定」→ 視窗重現**且在設定畫面** |
| F-4 不誤導航 | 在設定畫面點「顯示視窗」→ 應**停留**在設定畫面 |

---

## G. 轉檔輸出路徑（混合分隔符）

前端組路徑的方式是 `` `${basePath}VidBridge/Transcoded` ``，而 Windows 的下載路徑不以 `/` 結尾，所以會產生 `C:\Users\...\Downloads/VidBridge/Transcoded`（**混合分隔符**）。實務上 Windows API 與 ffmpeg 通常都接受，但沒實測過。

| 測試 | 預期 |
| --- | --- |
| 轉檔一支影片 | 輸出落在 `Downloads\VidBridge\Transcoded\`，且「開啟檔案」可正常開啟 |

---

## H. 資料庫 migration

若這台 Windows 上已有舊版 App 留下的資料庫（`%APPDATA%\com.vidbridge.app\vidbridge.db`），啟動新版會執行 migration 3（時間戳轉本地時間）。

| 測試 | 預期 |
| --- | --- |
| H-1 資料保留 | migration 前後下載歷史**筆數不變** |
| H-2 日期正確 | 歷史清單顯示的日期與實際下載日期一致 |
| H-3 不重複執行 | 再次啟動 App，時間**不再次位移** |

檢查指令（需安裝 sqlite3）：

```powershell
sqlite3 "$env:APPDATA\com.vidbridge.app\vidbridge.db" "SELECT version FROM _sqlx_migrations ORDER BY version;"
sqlite3 "$env:APPDATA\com.vidbridge.app\vidbridge.db" "SELECT id,status,created_at FROM download_history ORDER BY id;"
```

**建議先備份**該檔案再啟動。

---

## I. 自動更新 — 目前**還不能**在 Windows 測

change `fix-updater-signing-t1-bugs-and-remux` 的任務 7.1（提升版本號、推 `v*` tag 產出正式 release 與 `latest.json`）尚未執行，所以 updater endpoint 目前會回 404。

**預期行為**：啟動時 log 出現

```
[tauri_plugin_updater::updater][ERROR] update endpoint did not respond with a successful status code
```

且 App 正常啟動、不彈對話框。**這是正確的**（失敗可觀測、不阻擋啟動），不是新 bug。要真正測自動更新必須先完成 7.1。

---

## J. 下載影片的處理方式 — Windows 的自動判斷

Windows 以 `MFTEnumEx` 列舉 Media Foundation 註冊的視訊解碼器：找得到就回
supported、列舉成功但沒有就回 unsupported、列舉本身失敗才回 unknown。而**只有
supported 算許可**，所以 unsupported 與 unknown 都會保守地重新編碼。

**列舉刻意不過濾硬體旗標，軟體解碼也算 supported。** 要回答的問題是「這台機器能不能
解這個編碼」，軟體解碼確實能播。這使 Windows 比 macOS 寬鬆 —— macOS 用
`VTIsHardwareDecodeSupported`，只認硬體解碼 —— 因此**同一台機器在兩個系統上的答案
可能相反**（沒有 AV1 硬體解碼但裝了擴充的機器，Windows 回 supported、macOS 回
unsupported）。這是規格明文允許的差異，不是 bug：兩個方向都不會把「播不了」報成
supported。

要驗的是**自動判斷是否正確**。答案取決於機器裝了什麼，所以有兩種環境要各驗一次。

### J-A. 有 AV1 解碼能力的機器（Windows 11 24H2+ 內建，或已裝 AV1 Video Extension）

| 測試 | 步驟 | 預期 |
| --- | --- | --- |
| J-1 設定頁文字 | 「軟體設定 → 轉檔品質配置 → 下載影片的處理方式」選「自動判斷」 | 綠字 **「本機可解碼 AV1 → 保留原始畫質」**。若顯示「未能確認…」，表示列舉沒找到解碼器，先確認擴充是否真的安裝 |
| J-2 自動判斷保留原檔 | 下載一支 1080p 來源為 AV1 的 Facebook Reel | 狀態文字為「正在進行容器最佳化...」且幾乎瞬間完成；`ffprobe` 顯示輸出視訊 codec 為 **`av1`**、檔案大小接近下載到的原檔；**音訊位元率與來源相同** |
| J-3 覆寫仍有效 | 切到「相容優先」再下載同一支 | 狀態文字為「正在重新編碼以確保相容性...」；輸出 codec 為 **`h264`**，但音訊位元率**仍與來源相同**（AAC 來源一律 `-c:a copy`，重編只換視訊） |

J-2 保留下來的 AV1 檔案在**這台**機器上能播，但帶到不支援 AV1 的裝置上可能不行 ——
偵測只反映下載的這台機器。這正是「相容優先」存在的理由，設定頁也有對應警語。

### J-B. 沒有 AV1 解碼能力的機器

這一側**已由 CI 自動覆蓋**：`.github/workflows/test.yml` 在 `windows-latest` 上跑
`cargo test --lib`，而 GitHub runner 沒有 AV1 Video Extension，
`windows_answers_definitively_for_a_mapped_codec` 因此在該環境驗到的是 unsupported
那一側。不需要為此準備第二台機器。

若仍想在實機驗一次：移除 `Microsoft.AV1VideoExtension` 後**必須重啟 App** —— 能力
查詢每個行程只做一次並快取，不重啟不會反映。預期自動判斷回到「正在重新編碼以確保
相容性...」、輸出 codec 為 `h264`，設定頁文字變回「未能確認本機可解碼 AV1 →
重新編碼為 H.264」。驗完記得從 Microsoft Store 裝回。

### 對照基準：0.1.6（Windows 尚未實作自動判斷時）的實測數據

下表是 Windows 還一律回 unknown 時，在 Windows 11 25H2（build 26200）上以一支真實
Facebook Reel 量到的兩條路徑。來源為 **av1（Main）+ HE-AAC，1080×1920，9.38 秒，
2,068,245 bytes**。留在這裡當基準：J-2 現在應該落在右欄，而 0.1.6 落在左欄。

| | 重新編碼（0.1.6 的自動判斷） | 容器最佳化（現在的自動判斷） |
| --- | --- | --- |
| 輸出視訊 codec | h264 | **av1** |
| 音訊 | 88,709 bps HE-AAC，與來源完全相同（`-c:a copy`） | 同左 |
| 耗時 | 5.41 秒 | **0.07 秒**（77 倍差距） |
| 檔案大小 | 4,115,940 bytes（**比原檔大 99%**） | 2,068,245 bytes（與原檔相同） |

那個 +99% 值得記住：設計文件原本記的是「大 53%」，這支直式 Reel 實際翻倍。這就是
Windows 補上自動判斷所省下的代價。

驗證機的解碼能力細節，供對照：25H2、已裝 `Microsoft.AV1VideoExtension 2.0.24.0`
（**軟體**解碼），但 **沒有** AV1 硬體解碼路徑 —— `ffmpeg -hwaccel d3d11va` 對 AV1 回
"Your platform doesn't support hardware accelerated AV1 decoding"。也就是說這台機器
正是前面說的「兩個系統答案相反」的例子。

## 建置安裝檔（可選）

```powershell
npm run tauri build
```

產物在 `src-tauri\target\release\bundle\`（`msi\` 與 `nsis\`）。

注意：版本號三處目前都還是 **0.1.3**（`package.json`、`src-tauri\Cargo.toml`、`src-tauri\tauri.conf.json`）。要正式發佈需一併提升為 0.1.4 —— 那是任務 7.1，不要在驗證階段順手改。

---

## 回報方式

`npm run tauri dev` 的終端機輸出包含 Rust 端的 log（sqlx 查詢、updater、ffmpeg/yt-dlp 的 stderr）。失敗時請附上該段輸出。

前端的 `console.*` **不會**進入終端機（專案未安裝 JS 端 log plugin），需按 F12 或 Ctrl+Shift+I 開啟 WebView2 開發者工具查看。
