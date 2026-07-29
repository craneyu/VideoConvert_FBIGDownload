# VidBridge

VidBridge 是一款輕量、高效的影片下載與轉檔工具，支援 Facebook、Instagram 與 YouTube 影片。

## 🚀 立即下載

你可以前往 [GitHub Releases](https://github.com/craneyu/VideoConvert_FBIGDownload/releases) 下載最新版本。

*   **Windows**: 下載 `.msi` 或 `.exe` 安裝檔。
*   **macOS**: 依你的晶片選擇對應的 `.dmg`，**兩者不通用**：
    *   **Apple Silicon**（M1／M2／M3／M4…）→ `VidBridge_<版本>_aarch64.dmg`
    *   **Intel** → `VidBridge_<版本>_x64.dmg`

    不確定是哪一種？點左上角  →「關於這台 Mac」看「晶片」欄位；或在終端機執行 `uname -m`，輸出 `arm64` 是 Apple Silicon、`x86_64` 是 Intel。

    Apple Silicon 裝成 `x64` 版仍能透過 Rosetta 執行，但屬於轉譯執行而非原生，效能較差。

---

##  macOS 安裝須知 (重要)

由於本應用程式尚未向 Apple 註冊開發者帳號（年費項目），App 沒有經過 Apple 公證，macOS 會在第一次開啟時阻擋。

依 macOS 版本不同，你可能看到下列任一種訊息 —— **它們是同一件事**：

*   「Apple 無法驗證「VidBridge.app」是否為惡意軟體，它可能會損害你的 Mac 或危害你的隱私權。」（macOS 15 Sequoia 之後的措辭）
*   「無法打開「VidBridge」，因為無法驗證開發者。」
*   「App 已損毀，你應該將其移至垃圾桶。」

### 步驟

1.  下載對應晶片的 `.dmg`（見上一節），開啟後把 **VidBridge** 拖曳到 **應用程式** 資料夾。
2.  解除阻擋，以下**兩種擇一**即可：

    **方法 A —— 系統設定**

    1.  先雙擊開啟一次 App，讓系統記錄這次阻擋（此時只會看到警告，屬正常）。
    2.  前往「系統設定」→「隱私權與安全性」。
    3.  捲到最下方，會出現「已阻擋 "VidBridge" 以保護 Mac」，點選「**仍要打開**」。

    **方法 B —— 終端機（一次解決，不會再跳對話框）**

    ```bash
    xattr -dr com.apple.quarantine /Applications/VidBridge.app
    ```

    這行會移除 macOS 在下載時打上的隔離標記。App 位於 `/Applications` 且屬於你自己時**不需要 `sudo`**；若遇到權限不足，才在前面加上 `sudo` 並輸入電腦密碼。

> **注意：放行是綁定該 App 的，不是綁定電腦。**
> 重新安裝、更新版本、或從 Intel 版換成 Apple Silicon 版（反之亦然），都會被視為另一個 App，**必須重新放行一次**。這不代表新版本有問題。

> 較舊的教學常提到「按住 Control 點一下圖示 →『打開』」。這個捷徑在近期的 macOS 已不再適用，請改用上面兩種方法。

---

## ✨ 功能特色

- **智慧剪貼簿偵測**：自動識別剪貼簿中的 FB / IG / YT 影片網址。
- **精美 UI 介面**：採用現代化 macOS 風格設計，支援深色模式。
- **高效下載與轉檔**：支援多執行緒下載與影片格式轉換（開發中）。
- **自動更新**：內建自動更新機制，確保你始終使用最新版本。

## 🛠 開發指南

### 推薦 IDE 設定

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)。

### 啟動開發伺服器

```bash
npm install
npm run tauri dev
```

### 建置打包

```bash
npm run tauri build
```

## 📜 授權

MIT License
