use std::process::Command;
use std::path::Path;

#[tauri::command]
pub fn check_dependencies() -> Result<Vec<String>, String> {
    let mut missing = Vec::new();
    let tools = ["ffmpeg", "ffprobe", "yt-dlp"];

    for tool in tools {
        if find_tool_path(tool).is_none() {
            missing.push(tool.to_string());
        }
    }

    Ok(missing)
}

pub fn find_tool_path(name: &str) -> Option<String> {
    let common_paths = [
        format!("/opt/homebrew/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("/usr/bin/{}", name),
    ];

    for path in common_paths {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }

    // Final attempt: check if it's in the system PATH
    if Command::new(name).arg("--version").output().is_ok() {
        return Some(name.to_string());
    }

    None
}
