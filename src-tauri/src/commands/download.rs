use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use tauri::{AppHandle, Emitter};
use serde::Serialize;
use serde_json::Value;
use regex::Regex;
use crate::commands::utils::find_tool_path;

#[derive(Serialize, Clone)]
pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
    pub speed: String,
}

#[tauri::command]
pub async fn fetch_video_info(url: String) -> Result<String, String> {
    let yt_dlp_path = find_tool_path("yt-dlp").ok_or("yt-dlp not found")?;
    
    let output = Command::new(&yt_dlp_path)
        .arg("--dump-json")
        .arg("--no-playlist")
        .arg("--flat-playlist")
        .arg("--no-check-certificates")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .arg(&url)
        .output()
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(err);
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let title = json["title"].as_str().unwrap_or("Unknown Title").to_string();
    Ok(title)
}

#[tauri::command]
pub async fn download_video(app: AppHandle, id: String, url: String, download_dir: String, source: String) -> Result<String, String> {
    let yt_dlp_path = find_tool_path("yt-dlp").ok_or("yt-dlp not found")?;
    let ffmpeg_path = find_tool_path("ffmpeg").ok_or("ffmpeg not found")?;

    let target_dir = std::path::PathBuf::from(&download_dir)
        .join("VidBridge")
        .join(&source);
    
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let output_template = format!("{}/%(title)s.%(ext)s", target_dir.to_string_lossy());

    let mut child = Command::new(&yt_dlp_path)
        .arg("--newline")
        .arg("--progress")
        .arg("--no-check-certificates")
        .arg("--ffmpeg-location")
        .arg(&ffmpeg_path)
        .arg("-o")
        .arg(&output_template)
        .arg("--no-playlist")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stdout);
    let err_reader = BufReader::new(stderr);
    
    let re = Regex::new(r"\[download\]\s+(\d+\.?\d*)%").unwrap();
    let speed_re = Regex::new(r"at\s+([^\s]+)").unwrap();

    // Spawn a thread to capture stderr for detailed error reporting
    let mut full_error = String::new();
    
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(caps) = re.captures(&line) {
                let progress: f64 = caps[1].parse().unwrap_or(0.0);
                let speed = speed_re.captures(&line)
                    .map(|c| c[1].to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                
                let _ = app.emit("download-progress", ProgressPayload {
                    id: id.clone(),
                    progress,
                    speed,
                });
            }
        }
    }

    // Capture remaining stderr if process fails
    for line in err_reader.lines() {
        if let Ok(line) = line {
            full_error.push_str(&line);
            full_error.push('\n');
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for yt-dlp: {}", e))?;
    if !status.success() {
        return Err(format!("Download failed:\n{}", full_error));
    }

    let output = Command::new(&yt_dlp_path)
        .arg("--get-filename")
        .arg("-o")
        .arg(&output_template)
        .arg("--no-playlist")
        .arg(&url)
        .output()
        .map_err(|e| format!("Failed to get final filename: {}", e))?;
    
    let final_path = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::new()
    };

    Ok(final_path)
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}
