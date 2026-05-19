use std::process::Command;
use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub current_version: String,
    pub path: String,
}

#[derive(Serialize, Clone)]
pub struct DependencyCheckResult {
    pub platform: String,
    pub dependencies: Vec<DependencyStatus>,
}

/// Detect the current operating system platform.
fn detect_platform() -> String {
    if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "windows") {
        "windows".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Get the version string of an installed tool.
fn get_tool_version(tool_path: &str, tool_name: &str) -> String {
    let output = match tool_name {
        "ffmpeg" | "ffprobe" => Command::new(tool_path).arg("-version").output(),
        "yt-dlp" => Command::new(tool_path).arg("--version").output(),
        _ => Command::new(tool_path).arg("--version").output(),
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            match tool_name {
                "ffmpeg" | "ffprobe" => {
                    // Extract version like "ffmpeg version 6.1.1" or "ffmpeg version N-..."
                    stdout
                        .lines()
                        .next()
                        .and_then(|line| {
                            line.split_whitespace()
                                .nth(2)
                                .map(|v| v.to_string())
                        })
                        .unwrap_or_else(|| "unknown".to_string())
                }
                "yt-dlp" => {
                    // yt-dlp --version outputs just the version string like "2024.01.01"
                    stdout.trim().to_string()
                }
                _ => stdout.trim().to_string(),
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

#[tauri::command]
pub fn check_dependencies() -> Result<DependencyCheckResult, String> {
    let platform = detect_platform();
    let tools = ["ffmpeg", "ffprobe", "yt-dlp"];
    let mut dependencies = Vec::new();

    for tool in tools {
        let path = find_tool_path(tool);
        let (installed, version, tool_path) = match &path {
            Some(p) => {
                let ver = get_tool_version(p, tool);
                (true, ver, p.clone())
            }
            None => (false, String::new(), String::new()),
        };

        dependencies.push(DependencyStatus {
            name: tool.to_string(),
            installed,
            current_version: version,
            path: tool_path,
        });
    }

    Ok(DependencyCheckResult {
        platform,
        dependencies,
    })
}

/// Install or update dependencies based on the detected platform.
/// Returns a status message for each tool processed.
#[tauri::command]
pub async fn install_dependencies(tools: Vec<String>) -> Result<Vec<String>, String> {
    let platform = detect_platform();
    let mut results = Vec::new();

    for tool in &tools {
        let result = install_tool(&platform, tool);
        results.push(result);
    }

    Ok(results)
}

/// Install or update a single tool on the given platform.
fn install_tool(platform: &str, tool: &str) -> String {
    match platform {
        "macos" => install_tool_macos(tool),
        "windows" => install_tool_windows(tool),
        "linux" => install_tool_linux(tool),
        _ => format!("{}: 不支援的平台", tool),
    }
}

fn install_tool_macos(tool: &str) -> String {
    // Check if Homebrew is available
    let has_brew = Command::new("brew").arg("--version").output().is_ok();

    match tool {
        "ffmpeg" | "ffprobe" => {
            if !has_brew {
                return format!("{}: 需要先安裝 Homebrew (https://brew.sh)", tool);
            }
            // ffprobe is included with ffmpeg
            let pkg = "ffmpeg";
            let is_installed = find_tool_path("ffmpeg").is_some();

            let output = if is_installed {
                Command::new("brew").args(["upgrade", pkg]).output()
            } else {
                Command::new("brew").args(["install", pkg]).output()
            };

            match output {
                Ok(out) => {
                    if out.status.success() {
                        let msg = String::from_utf8_lossy(&out.stdout).to_string();
                        if msg.contains("already installed") || msg.contains("already up-to-date") {
                            format!("{}: 已是最新版本", tool)
                        } else {
                            format!("{}: 安裝/更新成功", tool)
                        }
                    } else {
                        let err = String::from_utf8_lossy(&out.stderr).to_string();
                        format!("{}: 安裝失敗 - {}", tool, err.lines().last().unwrap_or("unknown error"))
                    }
                }
                Err(e) => format!("{}: 執行失敗 - {}", tool, e),
            }
        }
        "yt-dlp" => {
            if !has_brew {
                // Try pip as fallback
                let pip_result = Command::new("pip3")
                    .args(["install", "--upgrade", "yt-dlp"])
                    .output();
                match pip_result {
                    Ok(out) if out.status.success() => {
                        return format!("{}: 透過 pip3 安裝/更新成功", tool);
                    }
                    _ => {
                        return format!("{}: 需要先安裝 Homebrew 或 pip3", tool);
                    }
                }
            }
            let is_installed = find_tool_path("yt-dlp").is_some();
            let output = if is_installed {
                Command::new("brew").args(["upgrade", "yt-dlp"]).output()
            } else {
                Command::new("brew").args(["install", "yt-dlp"]).output()
            };

            match output {
                Ok(out) => {
                    if out.status.success() {
                        let msg = String::from_utf8_lossy(&out.stdout).to_string();
                        if msg.contains("already installed") || msg.contains("already up-to-date") {
                            format!("{}: 已是最新版本", tool)
                        } else {
                            format!("{}: 安裝/更新成功", tool)
                        }
                    } else {
                        let err = String::from_utf8_lossy(&out.stderr).to_string();
                        format!("{}: 安裝失敗 - {}", tool, err.lines().last().unwrap_or("unknown error"))
                    }
                }
                Err(e) => format!("{}: 執行失敗 - {}", tool, e),
            }
        }
        _ => format!("{}: 不支援的工具", tool),
    }
}

fn install_tool_windows(tool: &str) -> String {
    // Check if winget is available
    let has_winget = Command::new("winget").arg("--version").output().is_ok();
    // Check if choco is available
    let has_choco = Command::new("choco").arg("--version").output().is_ok();

    match tool {
        "ffmpeg" | "ffprobe" => {
            if has_winget {
                let is_installed = find_tool_path("ffmpeg").is_some();
                let output = if is_installed {
                    Command::new("winget")
                        .args(["upgrade", "--id", "Gyan.FFmpeg", "--accept-source-agreements", "--accept-package-agreements"])
                        .output()
                } else {
                    Command::new("winget")
                        .args(["install", "--id", "Gyan.FFmpeg", "--accept-source-agreements", "--accept-package-agreements"])
                        .output()
                };
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 winget 安裝/更新成功", tool),
                    Ok(out) => {
                        let msg = String::from_utf8_lossy(&out.stdout).to_string();
                        if msg.contains("No available upgrade") || msg.contains("already installed") {
                            format!("{}: 已是最新版本", tool)
                        } else {
                            format!("{}: winget 安裝失敗，請手動安裝", tool)
                        }
                    }
                    Err(_) => format!("{}: winget 執行失敗", tool),
                }
            } else if has_choco {
                let is_installed = find_tool_path("ffmpeg").is_some();
                let output = if is_installed {
                    Command::new("choco").args(["upgrade", "ffmpeg", "-y"]).output()
                } else {
                    Command::new("choco").args(["install", "ffmpeg", "-y"]).output()
                };
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 choco 安裝/更新成功", tool),
                    _ => format!("{}: choco 安裝失敗", tool),
                }
            } else {
                format!("{}: 需要先安裝 winget 或 Chocolatey", tool)
            }
        }
        "yt-dlp" => {
            if has_winget {
                let is_installed = find_tool_path("yt-dlp").is_some();
                let output = if is_installed {
                    Command::new("winget")
                        .args(["upgrade", "--id", "yt-dlp.yt-dlp", "--accept-source-agreements", "--accept-package-agreements"])
                        .output()
                } else {
                    Command::new("winget")
                        .args(["install", "--id", "yt-dlp.yt-dlp", "--accept-source-agreements", "--accept-package-agreements"])
                        .output()
                };
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 winget 安裝/更新成功", tool),
                    Ok(out) => {
                        let msg = String::from_utf8_lossy(&out.stdout).to_string();
                        if msg.contains("No available upgrade") || msg.contains("already installed") {
                            format!("{}: 已是最新版本", tool)
                        } else {
                            format!("{}: winget 安裝失敗，請手動安裝", tool)
                        }
                    }
                    Err(_) => format!("{}: winget 執行失敗", tool),
                }
            } else {
                // Try pip as fallback on Windows
                let pip_result = Command::new("pip")
                    .args(["install", "--upgrade", "yt-dlp"])
                    .output();
                match pip_result {
                    Ok(out) if out.status.success() => format!("{}: 透過 pip 安裝/更新成功", tool),
                    _ => format!("{}: 需要先安裝 winget 或 pip", tool),
                }
            }
        }
        _ => format!("{}: 不支援的工具", tool),
    }
}

fn install_tool_linux(tool: &str) -> String {
    // Detect package manager
    let has_apt = Command::new("apt").arg("--version").output().is_ok();
    let has_dnf = Command::new("dnf").arg("--version").output().is_ok();
    let has_pacman = Command::new("pacman").arg("--version").output().is_ok();

    match tool {
        "ffmpeg" | "ffprobe" => {
            if has_apt {
                // Try without sudo first, then suggest sudo
                let output = Command::new("pkexec")
                    .args(["apt", "install", "-y", "ffmpeg"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 apt 安裝/更新成功", tool),
                    _ => format!("{}: 請在終端執行 'sudo apt install -y ffmpeg'", tool),
                }
            } else if has_dnf {
                let output = Command::new("pkexec")
                    .args(["dnf", "install", "-y", "ffmpeg"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 dnf 安裝/更新成功", tool),
                    _ => format!("{}: 請在終端執行 'sudo dnf install -y ffmpeg'", tool),
                }
            } else if has_pacman {
                let output = Command::new("pkexec")
                    .args(["pacman", "-S", "--noconfirm", "ffmpeg"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 pacman 安裝/更新成功", tool),
                    _ => format!("{}: 請在終端執行 'sudo pacman -S ffmpeg'", tool),
                }
            } else {
                format!("{}: 找不到支援的套件管理器 (apt/dnf/pacman)", tool)
            }
        }
        "yt-dlp" => {
            // pip3 is the most universal way on Linux
            let pip_result = Command::new("pip3")
                .args(["install", "--upgrade", "yt-dlp"])
                .output();
            match pip_result {
                Ok(out) if out.status.success() => format!("{}: 透過 pip3 安裝/更新成功", tool),
                _ => {
                    // Try package manager as fallback
                    if has_apt {
                        let output = Command::new("pkexec")
                            .args(["apt", "install", "-y", "yt-dlp"])
                            .output();
                        match output {
                            Ok(out) if out.status.success() => format!("{}: 透過 apt 安裝成功", tool),
                            _ => format!("{}: 請在終端執行 'pip3 install --upgrade yt-dlp'", tool),
                        }
                    } else {
                        format!("{}: 請在終端執行 'pip3 install --upgrade yt-dlp'", tool)
                    }
                }
            }
        }
        _ => format!("{}: 不支援的工具", tool),
    }
}

#[tauri::command]
pub fn read_clipboard_text(app: AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| e.to_string())
}

pub fn find_tool_path(name: &str) -> Option<String> {
    // Platform-specific common paths
    #[cfg(target_os = "macos")]
    let common_paths = vec![
        format!("/opt/homebrew/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("/usr/bin/{}", name),
    ];

    #[cfg(target_os = "windows")]
    let common_paths = vec![
        format!("C:\\ProgramData\\chocolatey\\bin\\{}.exe", name),
        format!("C:\\Program Files\\ffmpeg\\bin\\{}.exe", name),
        format!("C:\\ffmpeg\\bin\\{}.exe", name),
    ];

    #[cfg(target_os = "linux")]
    let common_paths = vec![
        format!("/usr/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("/snap/bin/{}", name),
    ];

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let common_paths: Vec<String> = vec![];

    for path in &common_paths {
        if Path::new(path).exists() {
            return Some(path.clone());
        }
    }

    // Final attempt: check if it's in the system PATH
    #[cfg(target_os = "windows")]
    let version_arg = "--version";
    #[cfg(not(target_os = "windows"))]
    let version_arg = "--version";

    let cmd_name = if cfg!(target_os = "windows") && !name.ends_with(".exe") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    if let Ok(output) = Command::new(&cmd_name).arg(version_arg).output() {
        if output.status.success() || !output.stdout.is_empty() {
            return Some(name.to_string());
        }
    }

    // On Unix, also try `which`
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }

    // On Windows, also try `where`
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("where").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }

    None
}
