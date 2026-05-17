# VidBridge Specification

VidBridge is a high-performance macOS GUI tool for video transcoding and downloading videos from social media platforms (Facebook and Instagram).

## 1. Vision & Goals
- **Performance & Stability**: Built with Rust to ensure a lightweight and reliable experience.
- **Modern UI**: A native-feeling macOS interface using Tauri.
- **Simplicity**: Easy-to-use batch processing and quick access via the macOS Menu Bar.
- **Open Source**: Community-driven development via GitHub.

## 2. Technical Stack
- **Framework**: [Tauri](https://tauri.app/) (Rust Backend + Web Frontend)
- **Frontend**: Svelte (or React) + Tailwind CSS for a macOS-native aesthetic.
- **Media Engine**: 
  - **Transcoding**: Relies on system-installed `ffmpeg` (e.g., via `brew install ffmpeg`).
  - **Downloading**: Uses `yt-dlp` compatible logic for extracting public media URLs.
- **Platform**: Exclusively optimized for macOS.

## 3. Core Capabilities

### 3.1 Video Transcoder
- **Batch Processing**: Drag and drop multiple files to convert simultaneously.
- **Format Support**: MP4, AVI, MOV, MKV, etc.
- **Quality Control**:
  - Resolution (e.g., 4K, 1080p, 720p).
  - Bitrate and FPS adjustments.
- **Progress Tracking**: Real-time progress bars for each task.
- **Notifications**: System-level notifications upon task completion.

### 3.2 Video Downloader
- **Source Support**: Facebook (Public videos), Instagram (Public videos/Reels).
- **Batch Download**: Input multiple URLs at once.
- **Download Options**:
  - Select desired resolution/quality.
  - Choose target format (defaulting to MP4).
- **File Management**:
  - **Custom Naming**: Use original titles or custom naming rules.
  - **Auto-Organization**: Automatically save videos into source-specific folders (e.g., `Downloads/VidBridge/Facebook/`).
- **Privacy Scope**: Only public videos are supported; no login/cookie handling is required for the MVP.

### 3.3 使用者介面 (GUI Features)
- **macOS 原生感**: 支援深色/淺色模式切換，符合 macOS 設計規範。
- **拖放支援 (Drag & Drop)**: 可直接拖曳影片檔至 App 開始轉檔任務。
- **任務管理面板**: 可隨時暫停、重啟或刪除排程中的下載/轉檔任務。

### 3.4 系統工具列 (System Tray / Menu Bar)
- **快速存取**: 常駐於 macOS 上方工具列，點擊圖示可快速切換主視窗顯示狀態。
- **功能選單 (Right-click Menu)**:
  - **快速下載**: 自動讀取剪貼簿中的網址並啟動下載。
  - **進度查看**: 顯示目前進行中任務的百分比摘要。
  - **設定**: 快速開啟設定視窗。
  - **結束程式**: 完全關閉應用程式。
- **背景運行**: 支援主視窗關閉後，程式仍於工具列背景執行。

## 4. User Experience (UX)
- **macOS Integration**: Support for Dark Mode and native file pickers.
- **Drag & Drop**: Direct file interaction for transcoding.
- **Task Management**: A dashboard to pause, resume, or cancel ongoing tasks.

## 5. Deployment & Maintenance
- **Open Source**: Hosted on GitHub for collaboration.
- **Update Mechanism**: Built-in update checks provided by Tauri.
- **Documentation**: Comprehensive README and help guides included.
