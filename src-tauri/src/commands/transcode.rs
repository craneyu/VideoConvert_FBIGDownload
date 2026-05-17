use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use tauri::{AppHandle, Emitter};
use serde::{Serialize, Deserialize};
use regex::Regex;
use tauri_plugin_notification::NotificationExt;
use crate::commands::utils::find_tool_path;

#[derive(Serialize, Clone)]
pub struct TranscodeProgress {
    pub id: String,
    pub progress: f64,
    pub time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscodeOptions {
    pub preset: String,     // "high", "balanced", "fast", "custom"
    pub resolution: String, // "original", "1080", "720", "480"
    pub codec: String,      // "h264", "h265"
}

#[tauri::command]
pub async fn transcode_video(
    app: AppHandle,
    id: String,
    input_path: String,
    output_path: String,
    options: TranscodeOptions,
) -> Result<String, String> {
    let ffmpeg_path = find_tool_path("ffmpeg").ok_or("ffmpeg not found")?;
    let ffprobe_path = find_tool_path("ffprobe").ok_or("ffprobe not found")?;

    println!("Transcoding using: {}", ffmpeg_path);
    
    // 1. Get total duration first for progress calculation
    let duration_output = Command::new(&ffprobe_path)
        .args(&[
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            &input_path,
        ])
        .output()
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    if !duration_output.status.success() {
        let err = String::from_utf8_lossy(&duration_output.stderr);
        return Err(format!("ffprobe error: {}", err));
    }

    let total_duration_str = String::from_utf8_lossy(&duration_output.stdout).trim().to_string();
    let total_duration: f64 = total_duration_str.parse().unwrap_or(0.0);

    if total_duration == 0.0 {
        return Err("Could not determine video duration".to_string());
    }

    // 2. Construct ffmpeg arguments
    let mut args = vec!["-i".to_string(), input_path, "-y".to_string()];

    match options.codec.as_str() {
        "h265" => args.extend(vec!["-c:v".to_string(), "libx265".to_string()]),
        _ => args.extend(vec!["-c:v".to_string(), "libx264".to_string()]),
    }

    match options.preset.as_str() {
        "high" => {
            args.extend(vec!["-crf".to_string(), "18".to_string(), "-preset".to_string(), "slow".to_string()]);
        }
        "fast" => {
            args.extend(vec!["-crf".to_string(), "28".to_string(), "-preset".to_string(), "veryfast".to_string()]);
        }
        _ => {
            args.extend(vec!["-crf".to_string(), "23".to_string(), "-preset".to_string(), "medium".to_string()]);
        }
    }

    if options.resolution != "original" {
        let scale = format!("scale=-2:{}", options.resolution);
        args.extend(vec!["-vf".to_string(), scale]);
    }

    args.push(output_path.clone());

    // 3. Start ffmpeg
    let mut child = Command::new(&ffmpeg_path)
        .args(&args)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let re = Regex::new(r"time=(\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();

    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(caps) = re.captures(&line) {
                let hours: f64 = caps[1].parse().unwrap_or(0.0);
                let minutes: f64 = caps[2].parse().unwrap_or(0.0);
                let seconds: f64 = caps[3].parse().unwrap_or(0.0);
                let ms: f64 = caps[4].parse().unwrap_or(0.0);
                
                let current_time = hours * 3600.0 + minutes * 60.0 + seconds + ms / 100.0;
                let progress = (current_time / total_duration * 100.0).min(100.0);
                let time_str = format!("{}:{}:{}.{}", &caps[1], &caps[2], &caps[3], &caps[4]);

                let _ = app.emit("transcode-progress", TranscodeProgress {
                    id: id.clone(),
                    progress,
                    time: time_str,
                });
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for ffmpeg: {}", e))?;
    if !status.success() {
        return Err("Transcoding failed".to_string());
    }

    let _ = app.notification()
        .builder()
        .title("VidBridge")
        .body("影片轉檔已完成！")
        .show();

    Ok(output_path)
}
