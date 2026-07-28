use std::collections::VecDeque;
use std::process::Stdio;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use serde::Serialize;
use serde_json::Value;
use regex::Regex;
use crate::commands::utils::{find_tool_path, hidden_cmd};

/// How many trailing stderr lines to keep from a spawned tool, so a failure can
/// report the real reason without letting output accumulate without bound.
const STDERR_TAIL_LINES: usize = 20;

/// Status text emitted with the post-download progress event. The text is how the
/// user (and acceptance testing) observes which post-processing path was taken.
const STATUS_REMUX: &str = "正在進行容器最佳化...";
const STATUS_REENCODE: &str = "正在重新編碼以確保相容性...";

#[derive(Serialize, Clone)]
pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
    pub speed: String,
}

/// What `ffprobe` reported about a downloaded file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub video_codec: String,
    /// `None` means the file carries no audio stream at all.
    pub audio_codec: Option<String>,
    pub width: u32,
    pub height: u32,
}

/// How a downloaded file should be turned into the final MP4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostProcessPlan {
    /// Copy the existing streams into a fast-start MP4 container.
    Remux,
    /// Re-encode the file to guarantee compatibility.
    ReEncode,
}

/// Parse `ffprobe -print_format json -show_streams` output.
///
/// Returns `None` when the output cannot be understood or carries no video
/// stream — callers treat that as "not known to be compatible".
pub fn parse_probe_json(raw: &str) -> Option<ProbeResult> {
    let root: Value = serde_json::from_str(raw).ok()?;
    let streams = root.get("streams")?.as_array()?;

    let video = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("video"))?;
    let audio_codec = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("audio"))
        .and_then(|s| s.get("codec_name").and_then(Value::as_str))
        .map(|c| c.to_string());

    Some(ProbeResult {
        video_codec: video.get("codec_name").and_then(Value::as_str)?.to_string(),
        audio_codec,
        width: u32::try_from(video.get("width").and_then(Value::as_i64)?).ok()?,
        height: u32::try_from(video.get("height").and_then(Value::as_i64)?).ok()?,
    })
}

/// Decide between remuxing and re-encoding.
///
/// Remuxing is only chosen when every whitelist condition holds: H.264 video,
/// AAC audio or no audio at all, and even pixel dimensions. `None` (unparseable
/// probe output) is deliberately conservative and re-encodes, because an
/// unplayable output is far worse than a slow one.
pub fn plan_post_processing(probe: Option<&ProbeResult>) -> PostProcessPlan {
    let Some(probe) = probe else {
        return PostProcessPlan::ReEncode;
    };

    let video_ok = probe.video_codec.eq_ignore_ascii_case("h264");
    let audio_ok = match probe.audio_codec.as_deref() {
        None => true,
        Some(codec) => codec.eq_ignore_ascii_case("aac"),
    };
    let dimensions_ok =
        probe.width > 0 && probe.height > 0 && probe.width % 2 == 0 && probe.height % 2 == 0;

    if video_ok && audio_ok && dimensions_ok {
        PostProcessPlan::Remux
    } else {
        PostProcessPlan::ReEncode
    }
}

/// Read `ffprobe` output for a file. Any failure yields `None`, which callers
/// treat as "re-encode to be safe" rather than failing the download.
fn probe_media(ffprobe_path: &str, path: &str) -> Option<ProbeResult> {
    let output = hidden_cmd(ffprobe_path)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_entries",
            "stream=codec_type,codec_name,width,height",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }
    parse_probe_json(&String::from_utf8_lossy(&output.stdout))
}

/// Continuously drain a child's stderr into a bounded ring buffer.
///
/// Draining matters as much as the content: an unread pipe fills up and blocks
/// the child process, which previously stalled downloads indefinitely.
fn spawn_stderr_collector(
    stderr: std::process::ChildStderr,
) -> (Arc<Mutex<VecDeque<String>>>, std::thread::JoinHandle<()>) {
    let tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_LINES)));
    let writer = Arc::clone(&tail);
    let handle = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let mut buf = writer.lock().unwrap();
            if buf.len() == STDERR_TAIL_LINES {
                buf.pop_front();
            }
            buf.push_back(line);
        }
    });
    (tail, handle)
}

/// Join the collected stderr tail into a single message.
fn collected_stderr(tail: &Arc<Mutex<VecDeque<String>>>) -> String {
    tail.lock()
        .map(|buf| buf.iter().cloned().collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

#[tauri::command]
pub async fn fetch_video_info(url: String) -> Result<String, String> {
    let yt_dlp_path = find_tool_path("yt-dlp").ok_or("yt-dlp not found")?;
    
    let output = hidden_cmd(&yt_dlp_path)
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

    let mut child = hidden_cmd(&yt_dlp_path)
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

    // Start draining stderr before reading stdout. An unread stderr pipe fills its
    // buffer and blocks yt-dlp, which previously stalled downloads indefinitely.
    let stderr = child.stderr.take().unwrap();
    let (stderr_tail, stderr_reader) = spawn_stderr_collector(stderr);

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
    let _ = stderr_reader.join();
    if !status.success() {
        // Surface yt-dlp's own explanation (private video, login required, region
        // block) instead of a fixed message that tells the user nothing.
        let detail = collected_stderr(&stderr_tail);
        return Err(if detail.trim().is_empty() {
            "Download failed during yt-dlp phase".to_string()
        } else {
            format!("Download failed during yt-dlp phase:\n{}", detail)
        });
    }

    // Get the final filename that yt-dlp actually used (it might have changed extension or added suffix)
    // Since we forced -o with a specific tmp name, it should be exactly that.
    
    // Step 2: Get the intended title for the final file
    let output = hidden_cmd(&yt_dlp_path)
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

    // Step 3: Produce the final MP4. Facebook and Instagram sources are usually
    // already H.264/AAC, where a container remux is effectively instant and
    // lossless; re-encoding is the fallback for anything else.
    let probe = find_tool_path("ffprobe")
        .as_deref()
        .and_then(|ffprobe_path| probe_media(ffprobe_path, &temp_output_str));
    let plan = plan_post_processing(probe.as_ref());

    let _ = app.emit("download-progress", ProgressPayload {
        id: id.clone(),
        progress: 95.0,
        speed: match plan {
            PostProcessPlan::Remux => STATUS_REMUX.to_string(),
            PostProcessPlan::ReEncode => STATUS_REENCODE.to_string(),
        },
    });

    let mut ffmpeg_cmd = hidden_cmd(&ffmpeg_path);
    ffmpeg_cmd
        .arg("-y") // Overwrite if exists
        .arg("-i")
        .arg(&temp_output_str)
        .arg("-map")
        .arg("0:v?") // Include video if present
        .arg("-map")
        .arg("0:a?"); // Include audio if present

    match plan {
        PostProcessPlan::Remux => {
            // Stream copy: no decode, no encode, no quality loss.
            ffmpeg_cmd.arg("-c").arg("copy");
        }
        PostProcessPlan::ReEncode => {
            ffmpeg_cmd
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
                .arg("128k");
        }
    }

    let ffmpeg_status = ffmpeg_cmd
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
        return Err(match plan {
            PostProcessPlan::Remux => "Container optimization (remux) failed".to_string(),
            PostProcessPlan::ReEncode => {
                "Compatibility optimization (re-encoding) failed".to_string()
            }
        });
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
    reveal_in_file_manager(&path)
}

/// Reveal a file in the platform's file manager, selecting it where supported.
///
/// Exactly one implementation is compiled per target. The catch-all variant
/// returns an error rather than a silent success, so an unsupported platform is
/// visible to the caller instead of looking like a working button that does nothing.
#[cfg(target_os = "macos")]
fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    hidden_cmd("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to reveal file in Finder: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    // explorer.exe exits with a non-zero code even when it succeeds, so only a
    // spawn failure is treated as an error.
    hidden_cmd("explorer")
        .arg(format!("/select,{}", path))
        .spawn()
        .map_err(|e| format!("Failed to reveal file in File Explorer: {}", e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    // The freedesktop FileManager1 interface selects the item; not every desktop
    // provides it, so fall back to opening the containing directory.
    let selected = hidden_cmd("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
            &format!("array:string:file://{}", path),
            "string:",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if selected {
        return Ok(());
    }

    let parent = std::path::Path::new(path)
        .parent()
        .ok_or_else(|| format!("No containing directory for {}", path))?;
    hidden_cmd("xdg-open")
        .arg(parent)
        .spawn()
        .map_err(|e| format!("Failed to open containing directory: {}", e))?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    Err(format!(
        "Revealing '{}' in a file manager is not supported on this platform",
        path
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(video: &str, audio: Option<&str>, width: u32, height: u32) -> ProbeResult {
        ProbeResult {
            video_codec: video.to_string(),
            audio_codec: audio.map(|a| a.to_string()),
            width,
            height,
        }
    }

    // The whitelist decision table from the video-download-engine spec.

    #[test]
    fn h264_aac_even_dimensions_is_remuxed() {
        let p = probe("h264", Some("aac"), 1920, 1080);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::Remux);
    }

    #[test]
    fn h264_without_audio_stream_is_remuxed() {
        let p = probe("h264", None, 1080, 1080);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::Remux);
    }

    #[test]
    fn non_aac_audio_is_re_encoded() {
        let p = probe("h264", Some("opus"), 1920, 1080);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::ReEncode);
    }

    #[test]
    fn non_h264_video_is_re_encoded() {
        let p = probe("vp9", Some("aac"), 1920, 1080);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::ReEncode);
    }

    #[test]
    fn odd_width_is_re_encoded() {
        let p = probe("h264", Some("aac"), 1919, 1080);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::ReEncode);
    }

    #[test]
    fn odd_height_is_re_encoded() {
        let p = probe("h264", Some("aac"), 1920, 1079);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::ReEncode);
    }

    #[test]
    fn unparseable_probe_output_is_re_encoded() {
        assert_eq!(plan_post_processing(None), PostProcessPlan::ReEncode);
    }

    // Codec names from ffprobe are compared case-insensitively.
    #[test]
    fn codec_comparison_ignores_case() {
        let p = probe("H264", Some("AAC"), 1920, 1080);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::Remux);
    }

    // Zero dimensions mean ffprobe gave us nothing usable.
    #[test]
    fn zero_dimensions_are_re_encoded() {
        let p = probe("h264", Some("aac"), 0, 0);
        assert_eq!(plan_post_processing(Some(&p)), PostProcessPlan::ReEncode);
    }

    #[test]
    fn parses_video_and_audio_streams() {
        let raw = r#"{"streams":[
            {"codec_type":"video","codec_name":"h264","width":1920,"height":1080},
            {"codec_type":"audio","codec_name":"aac"}
        ]}"#;
        assert_eq!(
            parse_probe_json(raw),
            Some(probe("h264", Some("aac"), 1920, 1080))
        );
    }

    #[test]
    fn parses_video_only_file_as_having_no_audio() {
        let raw = r#"{"streams":[
            {"codec_type":"video","codec_name":"h264","width":640,"height":480}
        ]}"#;
        let parsed = parse_probe_json(raw).expect("should parse");
        assert_eq!(parsed.audio_codec, None);
        assert_eq!(plan_post_processing(Some(&parsed)), PostProcessPlan::Remux);
    }

    #[test]
    fn rejects_output_without_a_video_stream() {
        let raw = r#"{"streams":[{"codec_type":"audio","codec_name":"aac"}]}"#;
        assert_eq!(parse_probe_json(raw), None);
    }

    #[test]
    fn rejects_malformed_output() {
        for raw in ["", "not json", "{}", r#"{"streams":"nope"}"#] {
            assert_eq!(parse_probe_json(raw), None, "input: {:?}", raw);
        }
    }

    #[test]
    fn video_stream_missing_dimensions_is_rejected() {
        let raw = r#"{"streams":[{"codec_type":"video","codec_name":"h264"}]}"#;
        assert_eq!(parse_probe_json(raw), None);
    }

    /// Verbatim output of the command `probe_media` builds, captured from
    /// ffprobe 8.0 on macOS:
    ///
    /// ffprobe -v error -print_format json \
    ///   -show_entries stream=codec_type,codec_name,width,height <file>
    ///
    /// Hand-written fixtures cannot catch a change in ffprobe's actual response
    /// shape. If this ever stops parsing, `plan_post_processing` silently falls
    /// back to re-encoding every download while every other test stays green —
    /// so the real payload is pinned here, including the sibling keys ffprobe
    /// emits alongside `streams`.
    const REAL_FFPROBE_OUTPUT: &str = r#"{
    "programs": [

    ],
    "stream_groups": [

    ],
    "streams": [
        {
            "codec_name": "h264",
            "codec_type": "video",
            "width": 640,
            "height": 480
        },
        {
            "codec_name": "aac",
            "codec_type": "audio"
        }
    ]
}"#;

    #[test]
    fn parses_real_ffprobe_output() {
        let parsed = parse_probe_json(REAL_FFPROBE_OUTPUT).expect("real ffprobe output must parse");
        assert_eq!(parsed, probe("h264", Some("aac"), 640, 480));
    }

    #[test]
    fn real_ffprobe_output_is_planned_as_remux() {
        // The end-to-end point of task 6.3: a real H.264/AAC file must take the
        // remux path, not fall back to re-encoding.
        let parsed = parse_probe_json(REAL_FFPROBE_OUTPUT).expect("real ffprobe output must parse");
        assert_eq!(plan_post_processing(Some(&parsed)), PostProcessPlan::Remux);
    }
}
