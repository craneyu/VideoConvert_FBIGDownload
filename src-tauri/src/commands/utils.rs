use serde::Serialize;
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
/// Windows API CREATE_NO_WINDOW flag — prevents spawned processes from creating a visible console window.
/// See: https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Create a Command that hides the console window on Windows.
pub fn hidden_cmd(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

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
        "ffmpeg" | "ffprobe" => hidden_cmd(tool_path).arg("-version").output(),
        "yt-dlp" => hidden_cmd(tool_path).arg("--version").output(),
        _ => hidden_cmd(tool_path).arg("--version").output(),
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
                        .and_then(|line| line.split_whitespace().nth(2).map(|v| v.to_string()))
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
    // Whitelist of allowed tool names to prevent command injection
    let allowed_tools = ["ffmpeg", "ffprobe", "yt-dlp"];

    for tool in &tools {
        if !allowed_tools.contains(&tool.as_str()) {
            results.push(format!("{}: 不支援的工具名稱", tool));
            continue;
        }
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
    let has_brew = hidden_cmd("brew").arg("--version").output().is_ok();

    match tool {
        "ffmpeg" | "ffprobe" => {
            if !has_brew {
                return format!("{}: 需要先安裝 Homebrew (https://brew.sh)", tool);
            }
            // ffprobe is included with ffmpeg
            let pkg = "ffmpeg";
            let is_installed = find_tool_path("ffmpeg").is_some();

            let output = if is_installed {
                hidden_cmd("brew").args(["upgrade", pkg]).output()
            } else {
                hidden_cmd("brew").args(["install", pkg]).output()
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
                        format!(
                            "{}: 安裝失敗 - {}",
                            tool,
                            err.lines().last().unwrap_or("unknown error")
                        )
                    }
                }
                Err(e) => format!("{}: 執行失敗 - {}", tool, e),
            }
        }
        "yt-dlp" => {
            if !has_brew {
                // Try pip as fallback
                let pip_result = hidden_cmd("pip3")
                    .args(["install", "--upgrade", "--user", "yt-dlp"])
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
                hidden_cmd("brew").args(["upgrade", "yt-dlp"]).output()
            } else {
                hidden_cmd("brew").args(["install", "yt-dlp"]).output()
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
                        format!(
                            "{}: 安裝失敗 - {}",
                            tool,
                            err.lines().last().unwrap_or("unknown error")
                        )
                    }
                }
                Err(e) => format!("{}: 執行失敗 - {}", tool, e),
            }
        }
        _ => format!("{}: 不支援的工具", tool),
    }
}

/// Check if winget output (combined stdout + stderr) indicates a successful or already-up-to-date result.
fn is_winget_success(out: &std::process::Output) -> bool {
    if out.status.success() {
        return true;
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = format!("{} {}", stdout, stderr);
    // winget returns non-zero for "no applicable update" or "already installed"
    combined.contains("No available upgrade")
        || combined.contains("No newer package versions")
        || combined.contains("already installed")
        || combined.contains("No applicable update")
        || combined.contains("Successfully installed")
        || combined.contains("Found an existing package")
}

/// Extract a user-friendly error message from winget output.
fn winget_error_message(out: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);
    combined
        .lines()
        .filter(|l| !l.trim().is_empty())
        .last()
        .unwrap_or("unknown error")
        .to_string()
}

fn install_tool_windows(tool: &str) -> String {
    // Check if winget is available (verify it actually runs, not just exists)
    let has_winget = hidden_cmd("winget")
        .args(["--version"])
        .output()
        .map(|o| o.status.success() || !o.stdout.is_empty())
        .unwrap_or(false);
    // Check if choco is available
    let has_choco = hidden_cmd("choco")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    match tool {
        "ffmpeg" | "ffprobe" => {
            if has_winget {
                let is_installed = find_tool_path("ffmpeg").is_some();
                let output = if is_installed {
                    hidden_cmd("winget")
                        .args([
                            "upgrade",
                            "--id",
                            "Gyan.FFmpeg",
                            "--accept-source-agreements",
                            "--accept-package-agreements",
                            "--disable-interactivity",
                        ])
                        .output()
                } else {
                    hidden_cmd("winget")
                        .args([
                            "install",
                            "--id",
                            "Gyan.FFmpeg",
                            "--accept-source-agreements",
                            "--accept-package-agreements",
                            "--disable-interactivity",
                        ])
                        .output()
                };
                match output {
                    Ok(out) if is_winget_success(&out) => {
                        let combined = format!(
                            "{} {}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                        if combined.contains("No available upgrade")
                            || combined.contains("No newer package versions")
                            || combined.contains("already installed")
                        {
                            format!("{}: 已是最新版本", tool)
                        } else {
                            format!("{}: 透過 winget 安裝/更新成功", tool)
                        }
                    }
                    Ok(out) => {
                        let err = winget_error_message(&out);
                        // Fallback to choco if winget fails
                        if has_choco {
                            let choco_out = if is_installed {
                                hidden_cmd("choco")
                                    .args(["upgrade", "ffmpeg", "-y"])
                                    .output()
                            } else {
                                hidden_cmd("choco")
                                    .args(["install", "ffmpeg", "-y"])
                                    .output()
                            };
                            match choco_out {
                                Ok(o) if o.status.success() => {
                                    format!("{}: 透過 choco 安裝/更新成功", tool)
                                }
                                _ => format!("{}: winget 安裝失敗 ({})", tool, err),
                            }
                        } else {
                            format!("{}: winget 安裝失敗 ({})", tool, err)
                        }
                    }
                    Err(e) => format!("{}: winget 執行失敗 - {}", tool, e),
                }
            } else if has_choco {
                let is_installed = find_tool_path("ffmpeg").is_some();
                let output = if is_installed {
                    hidden_cmd("choco")
                        .args(["upgrade", "ffmpeg", "-y"])
                        .output()
                } else {
                    hidden_cmd("choco")
                        .args(["install", "ffmpeg", "-y"])
                        .output()
                };
                match output {
                    Ok(out) if out.status.success() => {
                        format!("{}: 透過 choco 安裝/更新成功", tool)
                    }
                    Ok(out) => {
                        let err = String::from_utf8_lossy(&out.stderr).to_string();
                        format!(
                            "{}: choco 安裝失敗 - {}",
                            tool,
                            err.lines().last().unwrap_or("unknown error")
                        )
                    }
                    Err(e) => format!("{}: choco 執行失敗 - {}", tool, e),
                }
            } else {
                format!(
                    "{}: 需要先安裝 winget 或 Chocolatey。請以系統管理員身份開啟終端機並執行安裝",
                    tool
                )
            }
        }
        "yt-dlp" => {
            if has_winget {
                let is_installed = find_tool_path("yt-dlp").is_some();
                let output = if is_installed {
                    hidden_cmd("winget")
                        .args([
                            "upgrade",
                            "--id",
                            "yt-dlp.yt-dlp",
                            "--accept-source-agreements",
                            "--accept-package-agreements",
                            "--disable-interactivity",
                        ])
                        .output()
                } else {
                    hidden_cmd("winget")
                        .args([
                            "install",
                            "--id",
                            "yt-dlp.yt-dlp",
                            "--accept-source-agreements",
                            "--accept-package-agreements",
                            "--disable-interactivity",
                        ])
                        .output()
                };
                match output {
                    Ok(out) if is_winget_success(&out) => {
                        let combined = format!(
                            "{} {}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                        if combined.contains("No available upgrade")
                            || combined.contains("No newer package versions")
                            || combined.contains("already installed")
                        {
                            format!("{}: 已是最新版本", tool)
                        } else {
                            format!("{}: 透過 winget 安裝/更新成功", tool)
                        }
                    }
                    Ok(out) => {
                        let err = winget_error_message(&out);
                        // Fallback to pip
                        let pip_result = hidden_cmd("pip")
                            .args(["install", "--upgrade", "--user", "yt-dlp"])
                            .output();
                        match pip_result {
                            Ok(o) if o.status.success() => {
                                format!("{}: 透過 pip 安裝/更新成功", tool)
                            }
                            _ => format!("{}: winget 安裝失敗 ({})", tool, err),
                        }
                    }
                    Err(e) => format!("{}: winget 執行失敗 - {}", tool, e),
                }
            } else {
                // Try pip as fallback on Windows
                let pip_result = hidden_cmd("pip")
                    .args(["install", "--upgrade", "--user", "yt-dlp"])
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
    let has_apt = hidden_cmd("apt").arg("--version").output().is_ok();
    let has_dnf = hidden_cmd("dnf").arg("--version").output().is_ok();
    let has_pacman = hidden_cmd("pacman").arg("--version").output().is_ok();

    match tool {
        "ffmpeg" | "ffprobe" => {
            if has_apt {
                // Try without sudo first, then suggest sudo
                let output = hidden_cmd("pkexec")
                    .args(["apt", "install", "-y", "ffmpeg"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 apt 安裝/更新成功", tool),
                    _ => format!("{}: 請在終端執行 'sudo apt install -y ffmpeg'", tool),
                }
            } else if has_dnf {
                let output = hidden_cmd("pkexec")
                    .args(["dnf", "install", "-y", "ffmpeg"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => format!("{}: 透過 dnf 安裝/更新成功", tool),
                    _ => format!("{}: 請在終端執行 'sudo dnf install -y ffmpeg'", tool),
                }
            } else if has_pacman {
                let output = hidden_cmd("pkexec")
                    .args(["pacman", "-S", "--noconfirm", "ffmpeg"])
                    .output();
                match output {
                    Ok(out) if out.status.success() => {
                        format!("{}: 透過 pacman 安裝/更新成功", tool)
                    }
                    _ => format!("{}: 請在終端執行 'sudo pacman -S ffmpeg'", tool),
                }
            } else {
                format!("{}: 找不到支援的套件管理器 (apt/dnf/pacman)", tool)
            }
        }
        "yt-dlp" => {
            // pip3 is the most universal way on Linux
            let pip_result = hidden_cmd("pip3")
                .args(["install", "--upgrade", "--user", "yt-dlp"])
                .output();
            match pip_result {
                Ok(out) if out.status.success() => format!("{}: 透過 pip3 安裝/更新成功", tool),
                _ => {
                    // Try package manager as fallback
                    if has_apt {
                        let output = hidden_cmd("pkexec")
                            .args(["apt", "install", "-y", "yt-dlp"])
                            .output();
                        match output {
                            Ok(out) if out.status.success() => {
                                format!("{}: 透過 apt 安裝成功", tool)
                            }
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
    app.clipboard().read_text().map_err(|e| e.to_string())
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
    let common_paths = {
        let mut paths = vec![
            format!("C:\\ProgramData\\chocolatey\\bin\\{}.exe", name),
            format!("C:\\Program Files\\ffmpeg\\bin\\{}.exe", name),
            format!("C:\\ffmpeg\\bin\\{}.exe", name),
        ];
        // Add user-specific winget/scoop paths
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            paths.push(format!(
                "{}\\Microsoft\\WinGet\\Links\\{}.exe",
                local_app_data, name
            ));
            paths.push(format!("{}\\Microsoft\\WinGet\\Packages\\Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe\\ffmpeg-7.1.1-full_build\\bin\\{}.exe", local_app_data, name));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            // scoop install path
            paths.push(format!("{}\\scoop\\shims\\{}.exe", user_profile, name));
            // pip --user install path for yt-dlp
            paths.push(format!(
                "{}\\AppData\\Roaming\\Python\\Scripts\\{}.exe",
                user_profile, name
            ));
            paths.push(format!(
                "{}\\AppData\\Local\\Programs\\Python\\Scripts\\{}.exe",
                user_profile, name
            ));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.push(format!("{}\\ffmpeg\\bin\\{}.exe", program_files, name));
        }
        paths
    };

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
    let cmd_name = if cfg!(target_os = "windows") && !name.ends_with(".exe") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    if let Ok(output) = hidden_cmd(&cmd_name).arg("--version").output() {
        if output.status.success() || !output.stdout.is_empty() {
            return Some(name.to_string());
        }
    }

    // On Unix, also try `which`
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = hidden_cmd("which").arg(name).output() {
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
        if let Ok(output) = hidden_cmd("where").arg(name).output() {
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
