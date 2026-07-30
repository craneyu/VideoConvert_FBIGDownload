use std::collections::VecDeque;
use std::process::Stdio;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_sql::DbInstances;
use serde::Serialize;
use serde_json::Value;
use regex::Regex;
use crate::commands::concurrency::{run_blocking, SharedCpuBudget};
use crate::commands::utils::{find_tool_path, hidden_cmd};

/// How many trailing stderr lines to keep from a spawned tool, so a failure can
/// report the real reason without letting output accumulate without bound.
const STDERR_TAIL_LINES: usize = 20;

/// Status text emitted with the post-download progress event. The text is how the
/// user (and acceptance testing) observes which post-processing path was taken.
const STATUS_REMUX: &str = "正在進行容器最佳化...";
const STATUS_REENCODE: &str = "正在重新編碼以確保相容性...";
/// Reported while the download waits for a CPU permit.
///
/// Distinct from `STATUS_REENCODE` on purpose: nothing is happening to the file
/// yet, and several downloads can sit here at once. Without a wording that reads
/// as queued, they all appear stuck at the same progress value.
const STATUS_QUEUED_FOR_ENCODE: &str = "已下載完成，等待編碼中...";

/// Top of the band reserved for yt-dlp's own progress.
///
/// Named rather than inlined because the waiting-for-encode status is reported at
/// exactly this value — the file is fully downloaded, and no post-processing has
/// started.
const DOWNLOAD_PHASE_CEILING: f64 = 90.0;

/// Map yt-dlp's own percentage onto the network phase's band.
///
/// Clamped: yt-dlp can report slightly over 100% when it merges formats, and that
/// must not spill into the band post-processing reports in.
pub fn download_phase_progress(percent: f64) -> f64 {
    (percent / 100.0).clamp(0.0, 1.0) * DOWNLOAD_PHASE_CEILING
}

/// Is this plan expensive enough to be billed against the shared CPU budget?
///
/// Only re-encoding is. Remuxing copies streams without decoding or encoding, so
/// requiring a permit would make a near-instant operation queue behind an
/// expensive one for no benefit.
pub fn needs_cpu_permit(plan: PostProcessPlan) -> bool {
    match plan {
        PostProcessPlan::Remux => false,
        PostProcessPlan::ReEncode => true,
    }
}

/// The progress band reserved for post-processing. yt-dlp's own progress is
/// mapped onto 0–90, and 95 is emitted once the plan is known.
const POST_PROCESS_START: f64 = 95.0;
/// Held just below 100 so completion stays the only thing that reports done.
const POST_PROCESS_CEILING: f64 = 99.9;

/// Byte budget for a single path component.
///
/// macOS/APFS rejects components over 255 bytes with ENAMETOOLONG. We stay well
/// under that because the effective limit varies with encoding and normalisation,
/// and because a shortened name is harmless while a failed download is not.
const MAX_FILENAME_BYTES: usize = 200;

/// Shorten a file name so the component fits the filesystem limit.
///
/// Facebook and Instagram "titles" are the whole post description — 600+
/// characters is normal — and `yt-dlp --get-filename` sanitises illegal
/// characters but does not bound length. Truncation happens on a UTF-8 character
/// boundary so the result stays valid, and the extension is preserved because
/// ffmpeg picks the output container from it.
pub fn bound_filename(name: &str, max_bytes: usize) -> String {
    if name.len() <= max_bytes {
        return name.to_string();
    }

    // Only a short trailing segment counts as an extension — a dot inside a long
    // description must not be mistaken for one.
    const MAX_EXTENSION_BYTES: usize = 8;
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 && name.len() - dot <= MAX_EXTENSION_BYTES => {
            (&name[..dot], &name[dot..])
        }
        _ => (name, ""),
    };

    let budget = max_bytes.saturating_sub(extension.len());
    let mut end = budget.min(stem.len());
    while end > 0 && !stem.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}{}", stem[..end].trim_end(), extension)
}

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

/// Parse one line of ffmpeg's `-progress` stream into elapsed output seconds.
///
/// Only `out_time_us` is read. `out_time_ms` carries microseconds in several
/// ffmpeg releases, so treating it as milliseconds overshoots by a factor of 1000,
/// and `out_time` is a formatted timestamp that needs parsing of its own. Lines
/// for other keys, and the `N/A` that appears before the first frame is written,
/// yield `None`.
pub fn parse_progress_out_time(line: &str) -> Option<f64> {
    let micros: f64 = line.trim().strip_prefix("out_time_us=")?.trim().parse().ok()?;
    (micros.is_finite() && micros >= 0.0).then_some(micros / 1_000_000.0)
}

/// Parse `ffprobe -show_entries format=duration -of csv=p=0` output.
///
/// `N/A` is what ffprobe prints when the container does not state a duration, and
/// it must not become zero — a zero would make every progress value 95%.
pub fn parse_duration_output(raw: &str) -> Option<f64> {
    let seconds: f64 = raw.trim().parse().ok()?;
    (seconds.is_finite() && seconds > 0.0).then_some(seconds)
}

/// Map elapsed output time onto the post-processing band.
///
/// Clamped at both ends: ffmpeg's final report can exceed the probed duration
/// slightly, and an unknown duration reports the start of the band rather than a
/// fabricated value.
pub fn post_process_progress(out_time_secs: f64, duration_secs: f64) -> f64 {
    if !(duration_secs > 0.0) {
        return POST_PROCESS_START;
    }
    let ratio = (out_time_secs / duration_secs).clamp(0.0, 1.0);
    (POST_PROCESS_START + ratio * (100.0 - POST_PROCESS_START)).min(POST_PROCESS_CEILING)
}

/// Does `file_name` look like the temporary download for `temp_id`?
///
/// The output template cannot pin the extension. yt-dlp names the file after the
/// format it actually selected, so a YouTube source offering only VP9/Opus lands
/// as `<id>.tmp.webm` — and when the template asked for `.mp4`, older yt-dlp
/// appends its own extension instead, giving `<id>.tmp.mp4.webm`. Both shapes are
/// accepted; matching on the prefix is what makes this independent of the format.
///
/// In-progress artefacts are excluded so a partial file is never handed to ffmpeg.
pub fn is_temp_output(file_name: &str, temp_id: &str) -> bool {
    file_name.starts_with(&format!("{}.tmp.", temp_id))
        && !file_name.ends_with(".part")
        && !file_name.ends_with(".ytdl")
}

/// Locate the file yt-dlp produced, whatever extension it chose.
///
/// Assuming `<id>.tmp.mp4` made ffprobe and ffmpeg read a path that does not
/// exist: the download failed with "No such file or directory" after the bytes
/// had already been fetched, and cleanup missed the real file, leaving a
/// multi-gigabyte orphan in the download folder.
///
/// The largest match wins, so a leftover per-format fragment cannot be mistaken
/// for the merged result.
fn resolve_temp_output(dir: &std::path::Path, temp_id: &str) -> Option<std::path::PathBuf> {
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !is_temp_output(name, temp_id) {
            continue;
        }
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        match &best {
            Some((best_size, _)) if size <= *best_size => {}
            _ => best = Some((size, entry.path())),
        }
    }
    best.map(|(_, path)| path)
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

/// Read a file's duration in seconds, for turning ffmpeg's elapsed output time
/// into a percentage. Kept separate from `probe_media` because that answers a
/// different question — whether the streams are already compatible.
fn probe_duration_secs(ffprobe_path: &str, path: &str) -> Option<f64> {
    let output = hidden_cmd(ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }
    parse_duration_output(&String::from_utf8_lossy(&output.stdout))
}

/// Drain a reader into a bounded ring buffer of lines.
///
/// Splits on bytes rather than using `lines()`. `lines()` yields an error for any
/// line that is not valid UTF-8, and `map_while(Result::ok)` ended the loop there:
/// the drain thread exits while the child keeps writing, the pipe fills, and the
/// child blocks forever — the very stall the draining exists to prevent. Windows
/// makes that reachable, because tools write their messages in the console code
/// page (cp950 on a Traditional Chinese install) rather than UTF-8.
///
/// Replacement characters in an error message are an acceptable outcome. A
/// download that never finishes is not.
fn drain_lines<R: std::io::Read>(reader: R, tail: &Arc<Mutex<VecDeque<String>>>) {
    for chunk in BufReader::new(reader).split(b'\n') {
        let Ok(bytes) = chunk else { break };
        let line = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\r')
            .to_string();
        let mut buf = tail.lock().unwrap();
        if buf.len() == STDERR_TAIL_LINES {
            buf.pop_front();
        }
        buf.push_back(line);
    }
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
    let handle = std::thread::spawn(move || drain_lines(stderr, &writer));
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

/// What the network phase produced, and how it must be finished.
struct DownloadedFile {
    temp_output: std::path::PathBuf,
    final_output_str: String,
    plan: PostProcessPlan,
    /// `None` means post-processing cannot report incremental progress.
    duration_secs: Option<f64>,
}

/// Fetch the video and decide how it must be turned into the final MP4.
///
/// Network-bound work plus two short probes. Nothing here is billed against the
/// CPU budget, which is why the plan has to be known by the time this returns.
fn run_network_phase(
    app: &AppHandle,
    id: &str,
    url: &str,
    yt_dlp_path: &str,
    ffmpeg_path: &str,
    target_dir: &std::path::Path,
    temp_id: &str,
) -> Result<DownloadedFile, String> {
    // Step 1: Download using yt-dlp (Get best quality available)
    // We use a temporary filename to ensure we can re-encode it safely.
    // The extension is left to yt-dlp — see resolve_temp_output for why.
    let temp_template = target_dir.join(format!("{}.tmp.%(ext)s", temp_id));
    let temp_template_str = temp_template.to_string_lossy().to_string();

    let mut child = hidden_cmd(yt_dlp_path)
        .arg("--newline")
        .arg("--progress")
        // Without this yt-dlp writes in the console code page, so a message
        // containing non-ASCII text reaches us as invalid UTF-8. See drain_lines.
        .arg("--encoding")
        .arg("utf-8")
        .arg("--no-check-certificates")
        .arg("--ffmpeg-location")
        .arg(ffmpeg_path)
        .arg("-o")
        .arg(&temp_template_str)
        .arg("--no-playlist")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .arg(url)
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

                let _ = app.emit("download-progress", ProgressPayload {
                    id: id.to_string(),
                    progress: download_phase_progress(progress),
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

    // Which extension yt-dlp settled on is only knowable after the fact, so the
    // file has to be looked up rather than assumed.
    let temp_output = resolve_temp_output(target_dir, temp_id).ok_or_else(|| {
        format!(
            "yt-dlp reported success but left no {}.tmp.* file in {}",
            temp_id,
            target_dir.display()
        )
    })?;
    let temp_output_str = temp_output.to_string_lossy().to_string();

    // Step 2: Get the intended title for the final file
    let output = hidden_cmd(yt_dlp_path)
        .arg("--get-filename")
        .arg("-o")
        .arg("%(title)s.mp4") // Force .mp4 extension for final
        // The filename is printed on stdout in the console code page unless this
        // is set. On a Traditional Chinese Windows (cp950) a CJK title decoded as
        // UTF-8 became 65 U+FFFD replacement characters, and the video was saved
        // under that mojibake name; characters absent from the code page — emoji,
        // and the fullwidth solidus yt-dlp substitutes for "/" — were lost outright.
        .arg("--encoding")
        .arg("utf-8")
        .arg("--no-playlist")
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to get title: {}", e))?;

    let base_name = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        format!("{}.mp4", temp_id)
    };
    // yt-dlp sanitises illegal characters but not length, and a post description
    // used as a title easily exceeds the 255-byte component limit.
    let base_name = bound_filename(&base_name, MAX_FILENAME_BYTES);

    let final_output = target_dir.join(&base_name);

    // Facebook and Instagram sources are usually already H.264/AAC, where a
    // container remux is effectively instant and lossless; re-encoding is the
    // fallback for anything else. The decision is made here, before any permit is
    // requested, because only the re-encode path is billed.
    let ffprobe_path = find_tool_path("ffprobe");
    let probe = ffprobe_path
        .as_deref()
        .and_then(|ffprobe_path| probe_media(ffprobe_path, &temp_output_str));
    let plan = plan_post_processing(probe.as_ref());
    // Needed to turn ffmpeg's elapsed output time into a percentage. `None` simply
    // means post-processing reports no incremental progress.
    let duration_secs = ffprobe_path
        .as_deref()
        .and_then(|ffprobe_path| probe_duration_secs(ffprobe_path, &temp_output_str));

    Ok(DownloadedFile {
        temp_output,
        final_output_str: final_output.to_string_lossy().to_string(),
        plan,
        duration_secs,
    })
}

/// Produce the final MP4 from the downloaded file.
///
/// The caller holds a CPU permit for the whole of this when `file.plan` is a
/// re-encode.
fn run_post_process(
    app: &AppHandle,
    id: &str,
    ffmpeg_path: &str,
    file: &DownloadedFile,
) -> Result<String, String> {
    let temp_output_str = file.temp_output.to_string_lossy().to_string();
    let status_text = match file.plan {
        PostProcessPlan::Remux => STATUS_REMUX,
        PostProcessPlan::ReEncode => STATUS_REENCODE,
    };

    let _ = app.emit("download-progress", ProgressPayload {
        id: id.to_string(),
        progress: POST_PROCESS_START,
        speed: status_text.to_string(),
    });

    let mut ffmpeg_cmd = hidden_cmd(ffmpeg_path);
    ffmpeg_cmd
        .arg("-y") // Overwrite if exists
        .arg("-i")
        .arg(&temp_output_str)
        .arg("-map")
        .arg("0:v?") // Include video if present
        .arg("-map")
        .arg("0:a?"); // Include audio if present

    match file.plan {
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

    let mut ffmpeg_child = ffmpeg_cmd
        .arg("-sn") // Drop subtitles
        .arg("-dn") // Drop data streams
        .arg("-movflags")
        .arg("+faststart") // Enable streaming/fast playback
        // Machine-readable progress on stdout, and no duplicate stats on stderr —
        // which also keeps the stderr tail useful for reporting real failures.
        .arg("-progress")
        .arg("pipe:1")
        .arg("-nostats")
        .arg(&file.final_output_str)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("FFmpeg execution failed: {}", e))?;

    // Capture ffmpeg's stderr for the same reason as yt-dlp's: without it a
    // failure here reports only a fixed string, and the actual cause (for
    // example "File name too long") never reaches the user.
    let ffmpeg_stderr = ffmpeg_child.stderr.take().unwrap();
    let (ffmpeg_tail, ffmpeg_reader) = spawn_stderr_collector(ffmpeg_stderr);

    // Report progress while post-processing runs. Without this the UI sits on the
    // single 95% event for the whole re-encode — a 27 minute video took over 40
    // minutes on a real machine — which is indistinguishable from a hang.
    //
    // Reading to EOF here also drains the pipe, so ffmpeg cannot block on a full
    // one, and it doubles as waiting for the work to finish.
    if let Some(stdout) = ffmpeg_child.stdout.take() {
        for chunk in BufReader::new(stdout).split(b'\n') {
            let Ok(bytes) = chunk else { break };
            let Some(out_time) = parse_progress_out_time(&String::from_utf8_lossy(&bytes)) else {
                continue;
            };
            let Some(duration) = file.duration_secs else { continue };
            let _ = app.emit(
                "download-progress",
                ProgressPayload {
                    id: id.to_string(),
                    progress: post_process_progress(out_time, duration),
                    speed: status_text.to_string(),
                },
            );
        }
    }

    let ffmpeg_status = ffmpeg_child
        .wait()
        .map_err(|e| format!("Failed to wait for ffmpeg: {}", e))?;
    let _ = ffmpeg_reader.join();

    // Cleanup temp file
    let _ = std::fs::remove_file(&file.temp_output);

    if !ffmpeg_status.success() {
        let phase = match file.plan {
            PostProcessPlan::Remux => "Container optimization (remux) failed",
            PostProcessPlan::ReEncode => "Compatibility optimization (re-encoding) failed",
        };
        let detail = collected_stderr(&ffmpeg_tail);
        return Err(if detail.trim().is_empty() {
            phase.to_string()
        } else {
            format!("{}:\n{}", phase, detail)
        });
    }

    let _ = app.emit("download-progress", ProgressPayload {
        id: id.to_string(),
        progress: 100.0,
        speed: "完成".to_string(),
    });

    Ok(file.final_output_str.clone())
}

#[tauri::command]
pub async fn download_video(
    app: AppHandle,
    id: String,
    url: String,
    download_dir: String,
    source: String,
    auto_organize: bool,
    db_instances: State<'_, DbInstances>,
    cpu_budget: State<'_, SharedCpuBudget>,
) -> Result<String, String> {
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

    let temp_id = uuid::Uuid::new_v4().to_string();

    let file = {
        let app = app.clone();
        let id = id.clone();
        let url = url.clone();
        let yt_dlp_path = yt_dlp_path.clone();
        let ffmpeg_path = ffmpeg_path.clone();
        let target_dir = target_dir.clone();
        run_blocking("Download", move || {
            run_network_phase(
                &app,
                &id,
                &url,
                &yt_dlp_path,
                &ffmpeg_path,
                &target_dir,
                &temp_id,
            )
        })
        .await??
    };

    // The network slot is free from this point: the bytes are on disk. Only a
    // re-encode is billed against the shared CPU budget — a remux copies streams
    // and starts immediately, however busy the encoders are.
    let _permit = if needs_cpu_permit(file.plan) {
        // Announced before the wait, not after it. The frontend uses this to stop
        // counting the task as downloading, so the next queued download can start,
        // and the user sees a task that is queued rather than a progress bar that
        // stopped moving.
        let _ = app.emit(
            "download-progress",
            ProgressPayload {
                id: id.clone(),
                progress: DOWNLOAD_PHASE_CEILING,
                speed: STATUS_QUEUED_FOR_ENCODE.to_string(),
            },
        );

        let limit = crate::commands::settings::cpu_concurrency_or_default(&db_instances).await;
        match cpu_budget.acquire(limit).await {
            Ok(permit) => Some(permit),
            Err(error) => {
                // The download already succeeded, so its bytes are on disk. They
                // must not be left behind — a merged 4K video is several
                // gigabytes, and nothing else knows this file exists.
                let _ = std::fs::remove_file(&file.temp_output);
                return Err(error);
            }
        }
    } else {
        None
    };

    let app_for_post = app.clone();
    let id_for_post = id.clone();
    run_blocking("Post-processing", move || {
        run_post_process(&app_for_post, &id_for_post, &ffmpeg_path, &file)
    })
    .await?
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

/// Build the raw `/select` command line for `explorer.exe`.
///
/// The quotes have to wrap the path only. Passing `/select,<path>` as a normal
/// argument makes `Command` quote the whole thing once the path contains a space
/// — `"/select,C:\My Videos\clip.mp4"` — which explorer cannot parse: it silently
/// opens the Documents folder and selects nothing, so the button looks broken.
///
/// Deliberately not `#[cfg(target_os = "windows")]`, so the test below runs on
/// every platform. This bug shipped precisely because the Windows branch was
/// never executed — or even compiled — on the machine it was developed on.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn explorer_select_arg(path: &str) -> String {
    format!("/select,\"{}\"", path)
}

#[cfg(target_os = "windows")]
fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    // explorer.exe exits with a non-zero code even when it succeeds, so only a
    // spawn failure is treated as an error.
    hidden_cmd("explorer")
        .raw_arg(explorer_select_arg(path))
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

    // Which plans are billed against the shared CPU budget. Remuxing is a stream
    // copy — no decode, no encode — so making it queue behind a re-encode would
    // price a near-instant operation as if it were an expensive one.

    #[test]
    fn re_encoding_needs_a_cpu_permit() {
        assert!(needs_cpu_permit(PostProcessPlan::ReEncode));
    }

    #[test]
    fn remuxing_does_not_need_a_cpu_permit() {
        assert!(!needs_cpu_permit(PostProcessPlan::Remux));
    }

    #[test]
    fn the_permit_decision_follows_the_existing_plan() {
        // Fed from the existing decision function rather than from a hand-written
        // plan, so the two cannot drift apart: an H.264/AAC file is remuxed and
        // therefore unbilled, and anything else is re-encoded and billed.
        let compatible = probe("h264", Some("aac"), 1920, 1080);
        let incompatible = probe("av01", Some("opus"), 1920, 1080);

        assert!(!needs_cpu_permit(plan_post_processing(Some(&compatible))));
        assert!(needs_cpu_permit(plan_post_processing(Some(&incompatible))));
        // Unreadable probe output re-encodes to be safe, so it is billed.
        assert!(needs_cpu_permit(plan_post_processing(None)));
    }

    // The network phase's progress band. The waiting-for-encode status is reported
    // at its ceiling, so the value has to be a named quantity rather than a magic
    // number spread across the emit sites.

    #[test]
    fn download_progress_spans_the_network_band() {
        assert_eq!(download_phase_progress(0.0), 0.0);
        assert_eq!(download_phase_progress(50.0), 45.0);
        assert_eq!(download_phase_progress(100.0), DOWNLOAD_PHASE_CEILING);
    }

    #[test]
    fn download_progress_never_exceeds_its_band() {
        // yt-dlp has been observed reporting slightly over 100% on merged formats;
        // that must not spill into the band reserved for post-processing.
        assert_eq!(download_phase_progress(101.0), DOWNLOAD_PHASE_CEILING);
    }

    #[test]
    fn the_network_band_ends_below_the_post_processing_band() {
        // If these ever overlap, progress would move backwards when the encode
        // starts.
        assert!(
            DOWNLOAD_PHASE_CEILING < POST_PROCESS_START,
            "network band must end before post-processing begins"
        );
    }

    #[test]
    fn the_waiting_status_names_the_wait_rather_than_the_work() {
        // The user sees this while nothing is happening to their file yet, so it
        // has to read as queued, not as in progress.
        assert!(STATUS_QUEUED_FOR_ENCODE.contains("等待"), "{}", STATUS_QUEUED_FOR_ENCODE);
        assert_ne!(STATUS_QUEUED_FOR_ENCODE, STATUS_REENCODE);
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

    // Filename bounding. A Facebook reel whose "title" was the entire post
    // description produced a 1395-byte name and ffmpeg failed with
    // "File name too long"; macOS rejects any component over 255 bytes.

    #[test]
    fn short_name_is_left_alone() {
        assert_eq!(bound_filename("clip.mp4", 200), "clip.mp4");
    }

    #[test]
    fn name_exactly_at_the_limit_is_left_alone() {
        let name = format!("{}.mp4", "a".repeat(196));
        assert_eq!(name.len(), 200);
        assert_eq!(bound_filename(&name, 200), name);
    }

    #[test]
    fn long_ascii_name_is_truncated_and_keeps_its_extension() {
        let name = format!("{}.mp4", "a".repeat(500));
        let bounded = bound_filename(&name, 200);
        assert!(bounded.len() <= 200, "got {} bytes", bounded.len());
        assert!(bounded.ends_with(".mp4"));
    }

    #[test]
    fn long_multibyte_name_is_truncated_on_a_character_boundary() {
        // Each CJK character is 3 bytes, so a naive byte slice would split one
        // and panic.
        let name = format!("{}.mp4", "測".repeat(300));
        let bounded = bound_filename(&name, 200);
        assert!(bounded.len() <= 200, "got {} bytes", bounded.len());
        assert!(bounded.ends_with(".mp4"));
        // Valid UTF-8 with no replacement characters.
        assert!(!bounded.contains('\u{FFFD}'));
    }

    #[test]
    fn the_reel_title_that_broke_the_download_is_bounded() {
        // Shape of the real failure: a long description with inner dots and the
        // fullwidth separators yt-dlp substitutes for illegal characters.
        let title = format!(
            "京急線県立大学駅から徒歩3分 Y&Kスポーツアカデミー {} ｜ 山城裕之.mp4",
            "スポーツを楽しめるアカデミー".repeat(40)
        );
        assert!(title.len() > 1000, "fixture should reproduce the size");
        let bounded = bound_filename(&title, MAX_FILENAME_BYTES);
        assert!(bounded.len() <= MAX_FILENAME_BYTES, "got {} bytes", bounded.len());
        assert!(bounded.ends_with(".mp4"));
    }

    #[test]
    fn name_without_extension_is_still_bounded() {
        let bounded = bound_filename(&"b".repeat(400), 200);
        assert!(bounded.len() <= 200);
    }

    #[test]
    fn dots_inside_a_long_description_are_not_treated_as_an_extension() {
        // "…v1.2.3 something very long" must not have ".3 something very long"
        // mistaken for the extension.
        let name = format!("clip v1.2.3 {}.mp4", "x".repeat(400));
        let bounded = bound_filename(&name, 200);
        assert!(bounded.len() <= 200);
        assert!(bounded.ends_with(".mp4"));
    }

    #[test]
    fn truncation_does_not_leave_trailing_whitespace() {
        let name = format!("{} {}.mp4", "a".repeat(190), "b".repeat(100));
        let bounded = bound_filename(&name, 200);
        let stem = bounded.trim_end_matches(".mp4");
        assert_eq!(stem, stem.trim_end(), "stem should not end with whitespace");
    }

    // Post-processing progress. The UI previously sat on a single 95% event for the
    // entire re-encode, which users read as a hang.

    #[test]
    fn progress_line_is_read_as_seconds() {
        // 12.345678 s expressed the way ffmpeg -progress writes it.
        assert_eq!(parse_progress_out_time("out_time_us=12345678"), Some(12.345678));
    }

    #[test]
    fn progress_line_tolerates_trailing_carriage_return() {
        assert_eq!(parse_progress_out_time("out_time_us=1000000\r"), Some(1.0));
    }

    #[test]
    fn other_progress_keys_are_ignored() {
        // out_time_ms is deliberately not read: it holds microseconds in several
        // ffmpeg releases, so reading it as milliseconds overshoots 1000x.
        for line in [
            "out_time_ms=12345678",
            "out_time=00:00:12.345678",
            "frame=298",
            "progress=continue",
            "speed=1.02x",
        ] {
            assert_eq!(parse_progress_out_time(line), None, "{} should be ignored", line);
        }
    }

    #[test]
    fn progress_before_the_first_frame_is_ignored() {
        // ffmpeg emits N/A until it has written something.
        assert_eq!(parse_progress_out_time("out_time_us=N/A"), None);
    }

    #[test]
    fn duration_output_is_parsed() {
        // The real duration of the video that exposed the missing progress.
        assert_eq!(parse_duration_output("1656.181000\n"), Some(1656.181));
    }

    #[test]
    fn unknown_duration_does_not_become_zero() {
        // A zero would make every progress value 95% and divide-by-zero the ratio.
        for raw in ["N/A", "", "0", "0.000000", "-1"] {
            assert_eq!(parse_duration_output(raw), None, "{:?} should be rejected", raw);
        }
    }

    #[test]
    fn progress_spans_the_reserved_band() {
        assert_eq!(post_process_progress(0.0, 100.0), 95.0);
        assert_eq!(post_process_progress(50.0, 100.0), 97.5);
    }

    #[test]
    fn progress_never_reports_complete_on_its_own() {
        // Completion is reported by its own event; ffmpeg's last report can also
        // exceed the probed duration.
        assert_eq!(post_process_progress(100.0, 100.0), POST_PROCESS_CEILING);
        assert_eq!(post_process_progress(120.0, 100.0), POST_PROCESS_CEILING);
    }

    #[test]
    fn unknown_duration_reports_the_start_of_the_band() {
        // Better than fabricating a percentage from a duration we do not have.
        assert_eq!(post_process_progress(42.0, 0.0), 95.0);
    }

    // Draining a child's stderr. A cp950 error message used to end the iteration,
    // which let the pipe fill and the child block — an unfinishable download.

    /// "流暢" in Big5: valid in cp950, invalid as UTF-8.
    const BIG5_BYTES: [u8; 4] = [0xAC, 0x79, 0xB3, 0x74];

    fn drained(input: Vec<u8>) -> Vec<String> {
        let tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_LINES)));
        drain_lines(std::io::Cursor::new(input), &tail);
        collected_stderr(&tail).lines().map(str::to_string).collect()
    }

    #[test]
    fn draining_continues_past_a_line_that_is_not_utf8() {
        let mut input = b"before\n".to_vec();
        input.extend_from_slice(&BIG5_BYTES);
        input.extend_from_slice(b"\nafter\n");

        let lines = drained(input);

        assert_eq!(lines.len(), 3, "no line may be dropped: {:?}", lines);
        assert_eq!(lines[0], "before");
        assert_eq!(lines[2], "after");
        // The undecodable line survives as replacement characters, not as a gap.
        assert!(lines[1].contains('\u{FFFD}'), "got {:?}", lines[1]);
    }

    #[test]
    fn draining_strips_carriage_returns() {
        // yt-dlp and ffmpeg both emit CRLF on Windows.
        assert_eq!(drained(b"one\r\ntwo\r\n".to_vec()), vec!["one", "two"]);
    }

    #[test]
    fn draining_keeps_only_the_last_lines() {
        let input: Vec<u8> = (0..STDERR_TAIL_LINES + 3)
            .map(|i| format!("line{}\n", i))
            .collect::<String>()
            .into_bytes();

        let lines = drained(input);

        assert_eq!(lines.len(), STDERR_TAIL_LINES);
        assert_eq!(lines[0], "line3", "oldest lines should be dropped first");
    }

    // Locating yt-dlp's output. A YouTube download landed as
    // `<id>.tmp.mp4.webm` while ffprobe and ffmpeg were handed `<id>.tmp.mp4`,
    // so the download failed after fetching 2.37 GB and the file was orphaned.

    const TEMP_ID: &str = "0bced93f-cff1-4263-9b86-a39f5df97b08";

    #[test]
    fn temp_output_is_found_whatever_extension_yt_dlp_chose() {
        for ext in ["mp4", "webm", "mkv", "m4a"] {
            let name = format!("{}.tmp.{}", TEMP_ID, ext);
            assert!(is_temp_output(&name, TEMP_ID), "{} should match", name);
        }
    }

    #[test]
    fn the_youtube_download_that_failed_is_recognised() {
        // The exact name left on disk by the failing download.
        assert!(is_temp_output(
            "0bced93f-cff1-4263-9b86-a39f5df97b08.tmp.mp4.webm",
            TEMP_ID
        ));
    }

    #[test]
    fn incomplete_downloads_are_not_treated_as_the_output() {
        // Handing a .part file to ffmpeg would produce a truncated video.
        assert!(!is_temp_output(&format!("{}.tmp.webm.part", TEMP_ID), TEMP_ID));
        assert!(!is_temp_output(&format!("{}.tmp.mp4.ytdl", TEMP_ID), TEMP_ID));
    }

    #[test]
    fn another_downloads_temp_file_is_ignored() {
        // Concurrent downloads share the directory, so the id must be respected.
        let other = "ffffffff-0000-0000-0000-000000000000";
        assert!(!is_temp_output(&format!("{}.tmp.mp4", other), TEMP_ID));
    }

    #[test]
    fn the_final_video_is_not_treated_as_the_temp_file() {
        assert!(!is_temp_output("My Holiday Clip.mp4", TEMP_ID));
        assert!(!is_temp_output(&format!("{}.mp4", TEMP_ID), TEMP_ID));
    }

    // Revealing a file in Explorer. Measured on Windows 11: with the path passed
    // as a normal argument, `C:\Users\<user>\My Videos\clip.mp4` opened
    // `C:\Users\<user>\OneDrive\文件` and selected nothing.

    #[test]
    fn explorer_argument_quotes_only_the_path() {
        assert_eq!(
            explorer_select_arg(r"C:\Users\u\My Videos\clip.mp4"),
            "/select,\"C:\\Users\\u\\My Videos\\clip.mp4\""
        );
    }

    #[test]
    fn explorer_argument_keeps_the_switch_outside_the_quotes() {
        // The whole argument must never be quoted as one unit; that is the form
        // explorer silently fails on.
        let arg = explorer_select_arg(r"C:\a b\c.mp4");
        assert!(arg.starts_with("/select,\""), "got {}", arg);
        assert!(!arg.starts_with('"'), "switch must stay unquoted: {}", arg);
    }

    #[test]
    fn explorer_argument_is_the_same_shape_without_spaces() {
        // A path without spaces used to work by accident; quoting it too keeps a
        // single code path rather than two behaviours to reason about.
        assert_eq!(
            explorer_select_arg(r"C:\Downloads\clip.mp4"),
            "/select,\"C:\\Downloads\\clip.mp4\""
        );
    }

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
