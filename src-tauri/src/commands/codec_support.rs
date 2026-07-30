//! Whether the running platform can decode a given video codec.
//!
//! This exists so a downloaded file can keep its original video stream when the
//! platform can play it, instead of being re-encoded for compatibility it does not
//! need. Re-encoding AV1 to H.264 was measured at 53% larger, 200x slower, and
//! visibly lossy (SSIM 0.985) compared with copying the stream.
//!
//! The answer is three-state, not a boolean. Linux offers nothing that represents
//! "can the user's player decode this", so the honest answer there is "unknown" —
//! collapsing that into a boolean would force a choice between two dishonest
//! answers. Callers treat only `Supported` as permission to keep the original, so
//! a wrong guess always errs towards a file that plays.
//!
//! **This says nothing about where the file ends up.** Users AirDrop downloads to
//! phones, copy them to other machines, and upload them elsewhere. This answers
//! only "can *this* machine decode it", which makes it a reasonable default and not
//! a guarantee — which is why the policy that consumes it is user-overridable.

use std::collections::HashMap;
use std::sync::Mutex;

/// Whether the platform can decode a codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeSupport {
    /// The platform reported that it can decode this codec.
    Supported,
    /// The platform reported that it cannot.
    Unsupported,
    /// The platform could not be asked, or the query failed.
    Unknown,
}

impl DecodeSupport {
    /// True only for `Supported`.
    ///
    /// Named to make call sites read as the deliberate asymmetry it is: `Unknown`
    /// and `Unsupported` are handled identically, so no caller can accidentally
    /// treat "we could not tell" as permission.
    pub fn is_supported(self) -> bool {
        matches!(self, DecodeSupport::Supported)
    }
}

/// Map a decoder-enumeration outcome onto an answer.
///
/// Kept separate from any platform call so the mapping is testable everywhere:
/// `Some(true)` a decoder was found, `Some(false)` the lookup succeeded and found
/// none, `None` the lookup itself failed.
pub fn support_from_lookup(found: Option<bool>) -> DecodeSupport {
    match found {
        Some(true) => DecodeSupport::Supported,
        Some(false) => DecodeSupport::Unsupported,
        None => DecodeSupport::Unknown,
    }
}

/// The four-character code MP4 and VideoToolbox use for a codec.
///
/// Keyed off the names `ffprobe` reports in `codec_name`, compared without regard
/// to case. `None` means we have no mapping, which callers treat as unknown rather
/// than guessing at a code.
pub fn video_codec_fourcc(codec: &str) -> Option<u32> {
    const fn fourcc(s: &[u8; 4]) -> u32 {
        ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
    }
    if codec.eq_ignore_ascii_case("av1") {
        Some(fourcc(b"av01"))
    } else if codec.eq_ignore_ascii_case("h264") {
        Some(fourcc(b"avc1"))
    } else if codec.eq_ignore_ascii_case("hevc") || codec.eq_ignore_ascii_case("h265") {
        Some(fourcc(b"hvc1"))
    } else {
        None
    }
}

/// Memoised answers, keyed by the lowercased codec name.
fn cache() -> &'static Mutex<HashMap<String, DecodeSupport>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, DecodeSupport>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Run `query` at most once per codec, reusing the answer afterwards.
///
/// A platform's decode capability does not change while the process runs —
/// installing a decoder package needs an application restart before the platform
/// reports it — and a decoder lookup is not free enough to repeat for every
/// download.
///
/// A panic inside `query` becomes `Unknown`. A failed capability check must never
/// take a download down with it, and it must never be mistaken for support.
fn memoised(codec: &str, query: impl FnOnce() -> DecodeSupport) -> DecodeSupport {
    let key = codec.to_ascii_lowercase();

    // Not held across `query`: the lock only guards the map, and holding it while
    // calling into a system framework would serialise unrelated lookups.
    if let Some(cached) = cache().lock().ok().and_then(|c| c.get(&key).copied()) {
        return cached;
    }

    let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(query))
        .unwrap_or(DecodeSupport::Unknown);

    if let Ok(mut c) = cache().lock() {
        c.insert(key, answer);
    }
    answer
}

/// Can this platform decode `codec`?
///
/// `codec` uses the names `ffprobe` reports, for example `h264` or `av1`.
pub fn video_decode_support(codec: &str) -> DecodeSupport {
    memoised(codec, || platform_support(codec))
}

/// The video codecs the download pipeline is able to remux into MP4.
///
/// Deliberately closed: a codec absent from this list is re-encoded whatever the
/// policy says, because its playability inside an MP4 container is not predictable.
/// VP9 is the notable omission — legal in MP4, poorly supported in practice.
pub const REMUXABLE_VIDEO_CODECS: [&str; 2] = ["h264", "av1"];

/// Which remuxable codecs this machine reports it can decode.
///
/// Exposed to the frontend so the settings page can say what `auto` currently
/// resolves to. Without it `auto` gives the user no way to tell whether their
/// downloads are being kept or re-encoded.
#[tauri::command]
pub fn decodable_video_codecs() -> Vec<String> {
    REMUXABLE_VIDEO_CODECS
        .iter()
        .filter(|codec| video_decode_support(codec).is_supported())
        .map(|codec| codec.to_string())
        .collect()
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{video_codec_fourcc, DecodeSupport};

    // VideoToolbox answers only for *hardware* decoding, so a machine with software
    // decode but no hardware path is reported as unsupported and its downloads are
    // re-encoded. That under-reports, which is the safe direction: the output still
    // plays, only larger and slower, and the user can override the policy.
    #[link(name = "VideoToolbox", kind = "framework")]
    extern "C" {
        fn VTIsHardwareDecodeSupported(codec_type: u32) -> bool;
    }

    pub fn support(codec: &str) -> DecodeSupport {
        let Some(fourcc) = video_codec_fourcc(codec) else {
            return DecodeSupport::Unknown;
        };
        // Safe: the call takes a plain integer, returns a plain bool, and touches no
        // memory we own.
        let supported = unsafe { VTIsHardwareDecodeSupported(fourcc) };
        super::support_from_lookup(Some(supported))
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::DecodeSupport;

    /// Every platform other than macOS answers "unknown", so downloads are
    /// re-encoded exactly as they were before this capability existed.
    ///
    /// Linux is a limitation in principle: nothing at the system level represents
    /// "can the user's player decode this".
    ///
    /// **Windows is deliberately deferred, not overlooked.** Enumerating Media
    /// Foundation decoders is the intended approach — Windows 11 24H2 and later
    /// carry AV1 support, earlier versions need the user to install an extension, so
    /// the lookup would reflect the machine honestly. But `MFTEnumEx` is a COM API
    /// needing an extra dependency, and this project's Windows target cannot be
    /// built on macOS at all: `cargo check --target x86_64-pc-windows-msvc` fails
    /// while compiling `ring`'s C sources with "assert.h file not found". Shipping
    /// COM code that cannot be compiled or behaviourally tested where it was written
    /// is worse than answering "unknown" — the latter only forgoes an improvement,
    /// the former can hand the user a file that will not play.
    pub fn support(_codec: &str) -> DecodeSupport {
        DecodeSupport::Unknown
    }
}

fn platform_support(codec: &str) -> DecodeSupport {
    platform::support(codec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn only_supported_counts_as_permission() {
        assert!(DecodeSupport::Supported.is_supported());
        // The whole point of the three-state answer: these two are handled alike.
        assert!(!DecodeSupport::Unsupported.is_supported());
        assert!(!DecodeSupport::Unknown.is_supported());
    }

    #[test]
    fn a_failed_lookup_is_unknown_not_supported() {
        assert_eq!(support_from_lookup(Some(true)), DecodeSupport::Supported);
        assert_eq!(support_from_lookup(Some(false)), DecodeSupport::Unsupported);
        assert_eq!(support_from_lookup(None), DecodeSupport::Unknown);
    }

    // Codec names arrive from ffprobe, which has been observed reporting either case.

    #[test]
    fn fourcc_lookup_ignores_case() {
        let expected = video_codec_fourcc("av1");
        assert!(expected.is_some());
        for name in ["AV1", "av1", "Av1"] {
            assert_eq!(video_codec_fourcc(name), expected, "codec {}", name);
        }
    }

    #[test]
    fn fourcc_values_are_the_mp4_codes() {
        assert_eq!(video_codec_fourcc("av1"), Some(u32::from_be_bytes(*b"av01")));
        assert_eq!(video_codec_fourcc("h264"), Some(u32::from_be_bytes(*b"avc1")));
        assert_eq!(video_codec_fourcc("hevc"), Some(u32::from_be_bytes(*b"hvc1")));
    }

    #[test]
    fn an_unmapped_codec_has_no_fourcc() {
        // vp9 is deliberately absent: it is not a codec this project remuxes.
        assert_eq!(video_codec_fourcc("vp9"), None);
    }

    // Memoisation. The platform lookup must run once per codec, and a lookup that
    // blows up must not take the download with it.

    #[test]
    fn the_platform_lookup_runs_once_per_codec() {
        let calls = AtomicUsize::new(0);
        let query = || {
            calls.fetch_add(1, Ordering::SeqCst);
            DecodeSupport::Supported
        };

        // A codec name unique to this test, so the shared cache cannot be pre-warmed
        // by another test running in parallel.
        let codec = "memo-test-codec";
        assert_eq!(memoised(codec, query), DecodeSupport::Supported);
        assert_eq!(
            memoised(codec, || {
                calls.fetch_add(1, Ordering::SeqCst);
                DecodeSupport::Supported
            }),
            DecodeSupport::Supported
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "lookup should not repeat");
    }

    #[test]
    fn the_cache_key_ignores_case() {
        let calls = AtomicUsize::new(0);
        let bump = || {
            calls.fetch_add(1, Ordering::SeqCst);
            DecodeSupport::Supported
        };
        memoised("Case-Test-Codec", bump);
        memoised("case-test-codec", || {
            calls.fetch_add(1, Ordering::SeqCst);
            DecodeSupport::Supported
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_panicking_lookup_yields_unknown() {
        // A capability check that fails must not fail the download, and must not be
        // mistaken for support.
        let answer = memoised("panic-test-codec", || panic!("framework unavailable"));
        assert_eq!(answer, DecodeSupport::Unknown);
    }

    #[test]
    fn a_panicking_lookup_is_not_retried() {
        let calls = AtomicUsize::new(0);
        let codec = "panic-once-codec";
        let _ = memoised(codec, || {
            calls.fetch_add(1, Ordering::SeqCst);
            panic!("boom")
        });
        let _ = memoised(codec, || {
            calls.fetch_add(1, Ordering::SeqCst);
            panic!("boom")
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1, "unknown should be cached too");
    }

    // The platform branches. Exactly one is compiled per target.

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_platforms_answer_unknown_for_every_codec() {
        for codec in ["av1", "h264", "vp9", "anything"] {
            assert_eq!(platform::support(codec), DecodeSupport::Unknown, "codec {}", codec);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_answers_definitively_for_a_mapped_codec() {
        // Which way it answers depends on the machine, so the assertion is that it
        // commits to an answer rather than falling through to unknown.
        assert_ne!(platform::support("h264"), DecodeSupport::Unknown);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_answers_unknown_for_an_unmapped_codec() {
        // No four-character code means we cannot ask, which is not the same as "no".
        assert_eq!(platform::support("vp9"), DecodeSupport::Unknown);
    }
}

