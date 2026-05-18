# VidBridge

VidBridge 是一款輕量、高效的影片下載與轉檔工具，支援 Facebook 與 Instagram 影片。

## 🚀 立即下載

你可以前往 [GitHub Releases](https://github.com/craneyu/VideoConvert_FBIGDownload/releases) 下載最新版本。

*   **Windows**: 下載 `.msi` 或 `.exe` 安裝檔。
*   **macOS**: 下載 `.dmg` 檔案（支援 Intel 與 Apple Silicon）。

---

##  macOS 安裝須知 (重要)

由於本應用程式尚未向 Apple 註冊開發者帳號（年費項目），在 macOS 上安裝時會遇到系統安全阻擋。請按照以下步驟操作：

1.  下載並開啟 `.dmg` 檔案，將 **VidBridge** 拖曳至 **Applications (應用程式)** 資料夾。
2.  **第一次開啟時**：
    *   如果系統提示「無法開啟，因為無法驗證開發者」：
        *   請前往「系統設定」 > 「隱私權與安全性」。
        *   向下捲動找到 VidBridge，點選「仍要開啟」。
    *   如果系統提示「App 已損毀，你應該將其移至垃圾桶」：
        *   開啟「終端機 (Terminal)」。
        *   輸入以下指令並按 Enter：
            ```bash
            sudo xattr -d com.apple.quarantine /Applications/VidBridge.app
            ```
        *   輸入你的電腦密碼即可正常開啟。

---

## ✨ 功能特色

- **智慧剪貼簿偵測**：自動識別剪貼簿中的 FB/IG 影片網址。
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
