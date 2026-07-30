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

#[cfg(target_os = "windows")]
mod platform {
    use super::{support_from_lookup, DecodeSupport};
    use windows::core::GUID;
    use windows::Win32::Media::MediaFoundation::{
        IMFActivate, MFTEnumEx, MFMediaType_Video, MFVideoFormat_AV1, MFVideoFormat_H264,
        MFVideoFormat_HEVC, MFT_CATEGORY_VIDEO_DECODER, MFT_ENUM_FLAG, MFT_ENUM_FLAG_ASYNCMFT,
        MFT_ENUM_FLAG_HARDWARE, MFT_ENUM_FLAG_SYNCMFT, MFT_REGISTER_TYPE_INFO,
    };
    use windows::Win32::System::Com::CoTaskMemFree;

    /// Which kinds of decoder count as evidence that this machine can play a codec.
    ///
    /// **Not restricted to hardware, deliberately.** The question being answered is
    /// "can *this machine* decode it", and a software decoder decodes it. Restricting
    /// to hardware would answer a different question and, on a machine whose GPU has
    /// no AV1 path but which has the AV1 Video Extension installed, would report
    /// Unsupported for a file that plays — forgoing the whole improvement. macOS
    /// answers the narrower hardware-only question because VideoToolbox offers
    /// nothing else; that asymmetry is allowed, and both directions are safe because
    /// neither reports Supported for something it cannot play.
    ///
    /// `MFT_ENUM_FLAG_TRANSCODE_ONLY` is left out on purpose: a decoder marked
    /// transcode-only is not evidence the user can play the file.
    const ENUM_FLAGS: MFT_ENUM_FLAG = MFT_ENUM_FLAG(
        MFT_ENUM_FLAG_SYNCMFT.0 | MFT_ENUM_FLAG_ASYNCMFT.0 | MFT_ENUM_FLAG_HARDWARE.0,
    );

    /// The Media Foundation subtype for a codec named the way `ffprobe` names it.
    ///
    /// Kept separate from `video_codec_fourcc` rather than sharing one table: MP4
    /// sample-entry codes and MF subtypes disagree — h264 is `avc1` in an MP4 but
    /// `H264` here, hevc is `hvc1` there but `HEVC` here. Reusing the MP4 codes would
    /// enumerate a subtype nothing is registered for and report Unsupported for a
    /// codec the machine decodes.
    ///
    /// `None` means no mapping, which callers treat as unknown rather than guessing.
    fn video_subtype(codec: &str) -> Option<GUID> {
        if codec.eq_ignore_ascii_case("av1") {
            Some(MFVideoFormat_AV1)
        } else if codec.eq_ignore_ascii_case("h264") {
            Some(MFVideoFormat_H264)
        } else if codec.eq_ignore_ascii_case("hevc") || codec.eq_ignore_ascii_case("h265") {
            Some(MFVideoFormat_HEVC)
        } else {
            None
        }
    }

    /// Map a decoder count onto an answer.
    ///
    /// `Some(0)` and `None` are different answers and must stay different: the first
    /// is the platform saying "there is no decoder", the second is the enumeration
    /// failing so the platform never answered at all.
    fn support_from_count(count: Option<u32>) -> DecodeSupport {
        support_from_lookup(count.map(|found| found > 0))
    }

    /// How many registered decoders accept `subtype` as an input type.
    ///
    /// `None` if the enumeration itself failed. Only the count is used — no decoder
    /// is instantiated, because the presence of one is the whole answer and creating
    /// one would cost far more than the query is worth.
    fn decoder_count(subtype: GUID) -> Option<u32> {
        let input = MFT_REGISTER_TYPE_INFO {
            guidMajorType: MFMediaType_Video,
            guidSubtype: subtype,
        };
        let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
        let mut count: u32 = 0;

        // Safe: `input` outlives the call, both out-parameters are owned locals, and
        // the array the call allocates is released below before this returns. No COM
        // initialisation is needed — this is a registry-backed lookup, verified on
        // Windows 11 to succeed with neither CoInitializeEx nor MFStartup called.
        unsafe {
            MFTEnumEx(
                MFT_CATEGORY_VIDEO_DECODER,
                ENUM_FLAGS,
                Some(&input),
                None,
                &mut activates,
                &mut count,
            )
            .ok()?;

            if !activates.is_null() {
                // Each slot holds a reference we now own. Reading the Option out and
                // dropping it releases that reference; the array itself was allocated
                // by the callee, so it goes back through CoTaskMemFree.
                for i in 0..count as usize {
                    drop(std::ptr::read(activates.add(i)));
                }
                CoTaskMemFree(Some(activates as *const std::ffi::c_void));
            }
        }

        Some(count)
    }

    pub fn support(codec: &str) -> DecodeSupport {
        let Some(subtype) = video_subtype(codec) else {
            return DecodeSupport::Unknown;
        };
        support_from_count(decoder_count(subtype))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::commands::codec_support::DecodeSupport;

        // The enumeration outcome is mapped by a pure function so these four cases
        // are testable on any machine: a test must not depend on which decoders the
        // build agent happens to have installed.

        #[test]
        fn a_decoder_found_is_supported() {
            assert_eq!(support_from_count(Some(1)), DecodeSupport::Supported);
        }

        #[test]
        fn enumeration_finding_nothing_is_unsupported() {
            // Distinct from a failed lookup: the platform answered, and said no.
            assert_eq!(support_from_count(Some(0)), DecodeSupport::Unsupported);
        }

        #[test]
        fn a_failed_enumeration_is_unknown() {
            assert_eq!(support_from_count(None), DecodeSupport::Unknown);
        }

        #[test]
        fn an_unmapped_codec_name_is_unknown_without_enumerating() {
            // vp9 has no subtype mapping, so the platform is never asked. Unknown
            // rather than Unsupported: "we did not ask" is not "there is none".
            assert_eq!(video_subtype("vp9"), None);
            assert_eq!(support("vp9"), DecodeSupport::Unknown);
        }

        #[test]
        fn subtype_lookup_ignores_case() {
            for name in ["AV1", "av1", "Av1"] {
                assert_eq!(
                    video_subtype(name),
                    Some(windows::Win32::Media::MediaFoundation::MFVideoFormat_AV1),
                    "codec {}",
                    name
                );
            }
        }

        #[test]
        fn the_mapped_codecs_are_the_ones_the_pipeline_can_remux() {
            use windows::Win32::Media::MediaFoundation::{MFVideoFormat_AV1, MFVideoFormat_H264};
            assert_eq!(video_subtype("av1"), Some(MFVideoFormat_AV1));
            assert_eq!(video_subtype("h264"), Some(MFVideoFormat_H264));
        }

        // Locks in the decision that software decoding counts as support. If someone
        // narrows the enumeration to hardware decoders only, this fails — which is
        // the point: on a machine with no AV1 hardware path that change silently
        // turns every answer into Unsupported and the feature stops doing anything.
        #[test]
        fn the_enumeration_is_not_restricted_to_hardware_decoders() {
            use windows::Win32::Media::MediaFoundation::MFT_ENUM_FLAG_SYNCMFT;
            assert_ne!(
                ENUM_FLAGS.0 & MFT_ENUM_FLAG_SYNCMFT.0,
                0,
                "software decoders must stay in scope"
            );
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use super::DecodeSupport;

    /// Linux answers "unknown", so downloads are re-encoded exactly as they were
    /// before this capability existed.
    ///
    /// This is a limitation in principle rather than deferred work: nothing at the
    /// system level represents "can the user's player decode this". A desktop can
    /// have ffmpeg, a browser, and a media player with three different codec sets.
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

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[test]
    fn linux_answers_unknown_for_every_codec() {
        for codec in ["av1", "h264", "vp9", "anything"] {
            assert_eq!(platform::support(codec), DecodeSupport::Unknown, "codec {}", codec);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_answers_definitively_for_a_mapped_codec() {
        // Which way it answers depends on which decoders the machine has — a build
        // agent without the AV1 Video Extension answers Unsupported, a desktop with
        // it answers Supported — so the assertion is that it commits to an answer
        // instead of falling through to unknown the way it did before this platform
        // was implemented.
        assert_ne!(platform::support("av1"), DecodeSupport::Unknown);
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

