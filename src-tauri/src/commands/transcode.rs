use std::process::Stdio;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tauri::{AppHandle, Emitter};
use serde::{Serialize, Deserialize};
use regex::Regex;
use tauri_plugin_notification::NotificationExt;
use crate::commands::utils::{find_tool_path, hidden_cmd};

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

/// True when the output path names an MP4 container.
fn is_mp4_output(output_path: &str) -> bool {
    std::path::Path::new(output_path)
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("mp4"))
        .unwrap_or(false)
}

/// Build the `ffmpeg` argument list for a transcode job.
///
/// Extracted as a pure function so the codec and container rules — in particular
/// the H.265 codec tag — are unit-testable without running ffmpeg.
pub fn build_transcode_args(
    input_path: &str,
    output_path: &str,
    options: &TranscodeOptions,
) -> Vec<String> {
    // -progress pipe:2 forces frequent machine-readable progress lines on stderr.
    let mut args = vec![
        "-i".to_string(),
        input_path.to_string(),
        "-progress".to_string(),
        "pipe:2".to_string(),
        "-y".to_string(),
    ];

    match options.codec.as_str() {
        "h265" => {
            args.extend(vec!["-c:v".to_string(), "libx265".to_string()]);
            // Without the hvc1 tag, ffmpeg writes an hev1 tag that QuickTime and
            // macOS Preview refuse to play.
            if is_mp4_output(output_path) {
                args.extend(vec!["-tag:v".to_string(), "hvc1".to_string()]);
            }
        }
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

    // Name the audio encoder explicitly rather than relying on whatever default
    // the output container happens to pick.
    args.extend(vec![
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "192k".to_string(),
    ]);

    args.push(output_path.to_string());
    args
}

#[tauri::command]
pub async fn transcode_video(
    app: AppHandle,
    id: String,
    input_path: String,
    output_path: String,
    options: TranscodeOptions,
) -> Result<String, String> {
    println!("--- Transcoding Command Received ---");
    println!("Input: {}", input_path);
    println!("Output: {}", output_path);

    let ffmpeg_path = find_tool_path("ffmpeg").ok_or("ffmpeg not found")?;
    let ffprobe_path = find_tool_path("ffprobe").ok_or("ffprobe not found")?;

    if let Some(parent) = Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    println!("Step 1: Fetching duration with ffprobe...");
    let duration_output = hidden_cmd(&ffprobe_path)
        .args(&[
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            &input_path,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    let total_duration_str = String::from_utf8_lossy(&duration_output.stdout).trim().to_string();
    let total_duration: f64 = total_duration_str.parse().unwrap_or(0.0);
    println!("Total duration confirmed: {}s", total_duration);

    if total_duration == 0.0 {
        return Err("Could not determine video duration".to_string());
    }

    // 2. Construct ffmpeg arguments
    let args = build_transcode_args(&input_path, &output_path, &options);
    println!("Step 2: Starting ffmpeg with args: {:?}", args);

    let mut child = hidden_cmd(&ffmpeg_path)
        .args(&args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    
    // Updated regex to catch both standard output and -progress machine output
    let re_time = Regex::new(r"out_time=(\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();
    let re_std = Regex::new(r"time=(\d{2}):(\d{2}):(\d{2})\.(\d{2})").unwrap();

    println!("Step 3: Parsing ffmpeg output for progress...");
    for line in reader.lines() {
        if let Ok(line) = line {
            let mut captured = false;
            let mut time_match = re_time.captures(&line);
            
            if time_match.is_none() {
                time_match = re_std.captures(&line);
            }

            if let Some(caps) = time_match {
                let hours: f64 = caps[1].parse().unwrap_or(0.0);
                let minutes: f64 = caps[2].parse().unwrap_or(0.0);
                let seconds: f64 = caps[3].parse().unwrap_or(0.0);
                let ms: f64 = caps[4].parse().unwrap_or(0.0);
                
                let current_time = hours * 3600.0 + minutes * 60.0 + seconds + ms / 100.0;
                let progress = (current_time / total_duration * 100.0).min(99.9);
                let time_str = format!("{}:{}:{}.{}", &caps[1], &caps[2], &caps[3], &caps[4]);

                let _ = app.emit("transcode-progress", TranscodeProgress {
                    id: id.clone(),
                    progress,
                    time: time_str,
                });
                captured = true;
            }
            
            if !captured && line.contains("progress=end") {
                 let _ = app.emit("transcode-progress", TranscodeProgress {
                    id: id.clone(),
                    progress: 100.0,
                    time: "finished".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn options(codec: &str, preset: &str, resolution: &str) -> TranscodeOptions {
        TranscodeOptions {
            preset: preset.to_string(),
            resolution: resolution.to_string(),
            codec: codec.to_string(),
        }
    }

    /// Find a flag and return the value that follows it.
    fn value_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    }

    #[test]
    fn h265_into_mp4_carries_the_hvc1_tag() {
        let args = build_transcode_args("in.mov", "out.mp4", &options("h265", "balanced", "original"));
        assert_eq!(value_after(&args, "-c:v").as_deref(), Some("libx265"));
        assert_eq!(
            value_after(&args, "-tag:v").as_deref(),
            Some("hvc1"),
            "H.265 in MP4 needs the hvc1 tag or QuickTime cannot play it"
        );
    }

    #[test]
    fn h265_into_non_mp4_container_omits_the_tag() {
        let args = build_transcode_args("in.mov", "out.mkv", &options("h265", "balanced", "original"));
        assert_eq!(value_after(&args, "-c:v").as_deref(), Some("libx265"));
        assert!(!args.iter().any(|a| a == "-tag:v"));
    }

    #[test]
    fn h264_never_carries_the_hvc1_tag() {
        let args = build_transcode_args("in.mov", "out.mp4", &options("h264", "balanced", "original"));
        assert_eq!(value_after(&args, "-c:v").as_deref(), Some("libx264"));
        assert!(!args.iter().any(|a| a == "-tag:v"));
    }

    #[test]
    fn audio_encoder_is_always_stated_explicitly() {
        for codec in ["h264", "h265"] {
            for preset in ["high", "balanced", "fast"] {
                let args =
                    build_transcode_args("in.mov", "out.mp4", &options(codec, preset, "original"));
                assert_eq!(
                    value_after(&args, "-c:a").as_deref(),
                    Some("aac"),
                    "codec={} preset={}",
                    codec,
                    preset
                );
            }
        }
    }

    #[test]
    fn mp4_extension_match_is_case_insensitive() {
        let args = build_transcode_args("in.mov", "OUT.MP4", &options("h265", "balanced", "original"));
        assert_eq!(value_after(&args, "-tag:v").as_deref(), Some("hvc1"));
    }

    #[test]
    fn presets_map_to_expected_crf_values() {
        for (preset, crf) in [("high", "18"), ("fast", "28"), ("balanced", "23")] {
            let args = build_transcode_args("in.mov", "out.mp4", &options("h264", preset, "original"));
            assert_eq!(value_after(&args, "-crf").as_deref(), Some(crf), "preset={}", preset);
        }
    }

    #[test]
    fn original_resolution_adds_no_scale_filter() {
        let args = build_transcode_args("in.mov", "out.mp4", &options("h264", "balanced", "original"));
        assert!(!args.iter().any(|a| a == "-vf"));
    }

    #[test]
    fn explicit_resolution_adds_a_scale_filter() {
        let args = build_transcode_args("in.mov", "out.mp4", &options("h264", "balanced", "720"));
        assert_eq!(value_after(&args, "-vf").as_deref(), Some("scale=-2:720"));
    }

    #[test]
    fn output_path_is_the_last_argument() {
        let args = build_transcode_args("in.mov", "out.mp4", &options("h265", "high", "1080"));
        assert_eq!(args.last().map(String::as_str), Some("out.mp4"));
    }
}
