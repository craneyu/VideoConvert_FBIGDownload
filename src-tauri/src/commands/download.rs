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
pub async fn download_video(app: AppHandle, id: String, url: String, download_dir: String, source: String, auto_organize: bool) -> Result<String, String> {
    let yt_dlp_path = find_tool_path("yt-dlp").ok_or("yt-dlp not found")?;
    let ffmpeg_path = find_tool_path("ffmpeg").ok_or("ffmpeg not found")?;

    let target_dir = if auto_organize {
        std::path::PathBuf::from(&download_dir)
            .join("VidBridge")
            .join(&source)
    } else {
        std::path::PathBuf::from(&download_dir)
    };
    
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Step 1: Download using yt-dlp (Get best quality available)
    // We use a temporary filename to ensure we can re-encode it safely
    let temp_id = uuid::Uuid::new_v4().to_string();
    let temp_output = target_dir.join(format!("{}.tmp.mp4", temp_id));
    let temp_output_str = temp_output.to_string_lossy().to_string();

    let mut child = Command::new(&yt_dlp_path)
        .arg("--newline")
        .arg("--progress")
        .arg("--no-check-certificates")
        .arg("--ffmpeg-location")
        .arg(&ffmpeg_path)
        .arg("-o")
        .arg(&temp_output_str)
        .arg("--no-playlist")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let re = Regex::new(r"\[download\]\s+(\d+\.?\d*)%").unwrap();
    let speed_re = Regex::new(r"at\s+([^\s]+)").unwrap();

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(caps) = re.captures(&line) {
                let progress: f64 = caps[1].parse().unwrap_or(0.0);
                let speed = speed_re.captures(&line)
                    .map(|c| c[1].to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                
                // Map yt-dlp progress to 0-90% range to leave room for re-encoding
                let display_progress = progress * 0.9;
                let _ = app.emit("download-progress", ProgressPayload {
                    id: id.clone(),
                    progress: display_progress,
                    speed,
                });
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for yt-dlp: {}", e))?;
    if !status.success() {
        return Err("Download failed during yt-dlp phase".to_string());
    }

    // Get the final filename that yt-dlp actually used (it might have changed extension or added suffix)
    // Since we forced -o with a specific tmp name, it should be exactly that.
    
    // Step 2: Get the intended title for the final file
    let output = Command::new(&yt_dlp_path)
        .arg("--get-filename")
        .arg("-o")
        .arg("%(title)s.mp4") // Force .mp4 extension for final
        .arg("--no-playlist")
        .arg(&url)
        .output()
        .map_err(|e| format!("Failed to get title: {}", e))?;
    
    let base_name = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        format!("{}.mp4", temp_id)
    };

    let final_output = target_dir.join(&base_name);
    let final_output_str = final_output.to_string_lossy().to_string();

    // Step 3: Re-encode to strictly compatible H.264/AAC using ffmpeg
    let _ = app.emit("download-progress", ProgressPayload {
        id: id.clone(),
        progress: 95.0,
        speed: "正在進行相容性優化...".to_string(),
    });

    let ffmpeg_status = Command::new(&ffmpeg_path)
        .arg("-y") // Overwrite if exists
        .arg("-i")
        .arg(&temp_output_str)
        .arg("-map")
        .arg("0:v?") // Include video if present
        .arg("-map")
        .arg("0:a?") // Include audio if present
        .arg("-c:v")
        .arg("libx264")
        .arg("-profile:v")
        .arg("high")
        .arg("-level")
        .arg("4.0")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-vf")
        .arg("scale=trunc(iw/2)*2:trunc(ih/2)*2") // Force even dimensions for QuickTime
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("128k")
        .arg("-sn") // Drop subtitles
        .arg("-dn") // Drop data streams
        .arg("-movflags")
        .arg("+faststart") // Enable streaming/fast playback
        .arg(&final_output_str)
        .status()
        .map_err(|e| format!("FFmpeg execution failed: {}", e))?;

    // Cleanup temp file
    let _ = std::fs::remove_file(&temp_output);

    if !ffmpeg_status.success() {
        return Err("Compatibility optimization (re-encoding) failed".to_string());
    }

    let _ = app.emit("download-progress", ProgressPayload {
        id: id.clone(),
        progress: 100.0,
        speed: "完成".to_string(),
    });

    Ok(final_output_str)
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
