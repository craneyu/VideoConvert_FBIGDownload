# Windows 驗證清單

本文用於在 Windows 上驗證 `fix/tray-settings-navigation` 分支（PR #6）。

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
git checkout fix/tray-settings-navigation
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

**預期**：`cargo check` 無錯誤；`cargo test --lib` **40 passed**（測試本身與平台無關，數字應與 macOS 一致）。

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
| C-2 英文長標題 + 深路徑 | 下載路徑設為層數較深、名稱較長的資料夾，下載一支**英文**長標題影片 | **這一項可能失敗**。若出現路徑過長錯誤，請附上完整錯誤訊息 |

若 C-2 失敗，修法是把預算改成同時考慮「完整路徑字元數」而非只看檔名位元組數；請回報後再處理，不要自行加大預算。

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

## J. 下載影片的處理方式 — Windows 的自動判斷**尚未實作**

macOS 以 VideoToolbox 查詢平台能否解碼 AV1，能解就直接保留原始串流（免重新編碼、
畫質無損、檔案更小）。**Windows 目前一律回「未知」**，因此自動判斷會保守地走重新
編碼 —— 行為與此功能加入前相同。

之所以延後：理想做法是列舉 Media Foundation 的解碼器，但 `MFTEnumEx` 是 COM API，
需要額外依賴，而此專案的 Windows target 無法在 macOS 上編譯（見 A 節），所以那段
程式碼在撰寫的機器上既無法編譯也無法驗證行為。誠實回「未知」只是少拿到改善；寫下
未經驗證的 COM 程式碼則可能產出播不了的檔案。

要驗的是「現況是否如預期」，而不是自動判斷是否正確：

1. 開啟「軟體設定 → 轉檔品質配置」，找到「下載影片的處理方式」。
2. 選「自動判斷」，確認說明文字為 **「未能確認本機可解碼 AV1 → 重新編碼為 H.264」**。
   若顯示「本機可解碼 AV1」，表示 Windows 分支被誤接到 macOS 實作，屬 bug。
3. 下載一支 Facebook Reel（1080p 來源為 AV1），確認：
   - 後處理狀態文字為「正在重新編碼以確保相容性...」
   - 以 `ffprobe` 檢查輸出，視訊 codec 為 `h264`
   - **音訊位元率與來源相同**（來源已是 AAC 時不應被重新編碼）
4. 切到「保留原檔」再下載同一支，確認：
   - 狀態文字為「正在進行容器最佳化...」且幾乎瞬間完成
   - 輸出視訊 codec 為 `av1`、檔案大小接近下載到的原檔
   - 在該台 Windows 上嘗試播放該檔案。**播不了是預期的**（除非是 Windows 11 24H2+
     或已安裝 AV1 Video Extension）—— 這正是自動判斷保守走重編的理由。

之後若要補上 Windows 的自動判斷，需要在有／無 AV1 解碼支援的兩種 Windows 環境各驗
一次「自動判斷」的解析結果與實際輸出 codec。

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
