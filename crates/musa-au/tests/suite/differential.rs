//! The hosted crossing renders what native preparation renders.
//!
//! `06-daw-boundary.md` §4 keeps host block size out of the music, and cites
//! Theorem R1-batch: a different partition of the same frames is
//! unobservable. Nothing about that is automatic once a C ABI, a decode step,
//! and a host-chosen block size are in the path, so it is measured here.

use std::ffi::CString;

use musa_au::{
    MUSA_AU_EVENT_NOTE_OFF, MUSA_AU_EVENT_NOTE_ON, MusaAuEvent, musa_au_instrument_release,
    musa_au_preparation_identity_music, musa_au_preparation_identity_part, musa_au_preparation_message,
    musa_au_preparation_ok, musa_au_preparation_release, musa_au_preparation_take, musa_au_prepare, musa_au_render,
    musa_au_rendered_frames, musa_au_reset,
};

pub(crate) const RATE: u32 = 48_000;

/// The fixture piece. Written out rather than taken from a template so the
/// part this suite renders is named here, where the tests can see it.
pub(crate) const PIECE: &str = "\
piece \"Hosted\" {
    tempo quarter = 96;
    meter 4/4;
    key c major;

    score {
        part piano {
            clef treble;

            voice upper {
                c4/4
                e4/4
                g4/4
                c5/4
            }
        }
    }
}
";

/// A project of one piece on disk, and the strings a C caller passes.
pub(crate) struct Fixture {
    _root: tempfile::TempDir,
    pub(crate) project: CString,
    pub(crate) part: CString,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().expect("a temporary directory");
        let path = root.path().join("piece.musa");
        std::fs::write(&path, PIECE).expect("the piece is written");
        Self {
            project: CString::new(path.to_string_lossy().into_owned()).expect("no NUL in a temporary path"),
            part: c"piano".to_owned(),
            _root: root,
        }
    }
}

/// A prepared instrument, or the reason there is not one.
pub(crate) fn prepare(fixture: &Fixture) -> *mut musa_au::MusaAuInstrument {
    let preparation =
        unsafe { musa_au_prepare(fixture.project.as_ptr(), std::ptr::null(), fixture.part.as_ptr(), RATE) };
    assert!(!preparation.is_null(), "preparation is never null");
    let ok = unsafe { musa_au_preparation_ok(preparation) };
    assert_eq!(ok, 1, "{}", unsafe { message(preparation) });
    let instrument = unsafe { musa_au_preparation_take(preparation) };
    assert!(!instrument.is_null());
    unsafe { musa_au_preparation_release(preparation) };
    instrument
}

pub(crate) unsafe fn message(preparation: *const musa_au::MusaAuPreparation) -> String {
    unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_message(preparation)) }
        .to_string_lossy()
        .into_owned()
}

/// Render `frames` in blocks of `block`, applying `events` at absolute
/// offsets from the start.
fn render(instrument: *mut musa_au::MusaAuInstrument, events: &[MusaAuEvent], frames: usize, block: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(frames.saturating_mul(2));
    let mut left = vec![0.0f32; block];
    let mut right = vec![0.0f32; block];
    let mut start = 0usize;
    while start < frames {
        let count = block.min(frames - start);
        let end = start + count;
        let inside: Vec<MusaAuEvent> = events
            .iter()
            .filter(|event| (event.frame as usize) >= start && (event.frame as usize) < end)
            .map(|event| MusaAuEvent {
                frame: event.frame - u32::try_from(start).expect("a block start fits"),
                ..*event
            })
            .collect();
        left[..count].fill(0.0);
        right[..count].fill(0.0);
        unsafe {
            musa_au_render(
                instrument,
                if inside.is_empty() {
                    std::ptr::null()
                } else {
                    inside.as_ptr()
                },
                u32::try_from(inside.len()).expect("a block holds few events"),
                left.as_mut_ptr(),
                right.as_mut_ptr(),
                u32::try_from(count).expect("a block fits"),
            );
        }
        for frame in 0..count {
            out.push(left[frame]);
            out.push(right[frame]);
        }
        start = end;
    }
    out
}

fn phrase() -> Vec<MusaAuEvent> {
    vec![
        MusaAuEvent {
            frame: 0,
            voice: 1,
            kind: MUSA_AU_EVENT_NOTE_ON,
            data1: 60,
            data2: 100,
            reserved: 0,
        },
        MusaAuEvent {
            frame: 733,
            voice: 2,
            kind: MUSA_AU_EVENT_NOTE_ON,
            data1: 67,
            data2: 64,
            reserved: 0,
        },
        MusaAuEvent {
            frame: 1500,
            voice: 1,
            kind: MUSA_AU_EVENT_NOTE_OFF,
            data1: 60,
            data2: 0,
            reserved: 0,
        },
    ]
}

/// Every host block partition of the same frames sounds the same.
#[test]
fn a_block_partition_is_unobservable() {
    let fixture = Fixture::new();
    let events = phrase();
    let frames = 4096;

    let whole = {
        let instrument = prepare(&fixture);
        let rendered = render(instrument, &events, frames, frames);
        unsafe { musa_au_instrument_release(instrument) };
        rendered
    };
    assert_eq!(whole.len(), frames * 2);

    for block in [1usize, 7, 64, 256, 511, 1024] {
        let instrument = prepare(&fixture);
        let rendered = render(instrument, &events, frames, block);
        unsafe { musa_au_instrument_release(instrument) };
        assert_eq!(
            rendered, whole,
            "a block size of {block} frames changed what the instrument sounded like"
        );
    }
}

/// The same events through the session's own hosted instrument produce the
/// same frames as the ABI does.
///
/// This is the differential the prompt asks for: the ABI is a crossing, not a
/// second renderer, and the only way to keep it one is to compare.
#[test]
fn the_abi_renders_what_the_session_renders() {
    let fixture = Fixture::new();
    let events = phrase();
    let frames = 2048;

    let through_abi = {
        let instrument = prepare(&fixture);
        let rendered = render(instrument, &events, frames, 256);
        unsafe { musa_au_instrument_release(instrument) };
        rendered
    };

    let mut native = musa_project::open_hosted_instrument(&musa_project::HostedRequest {
        project: std::path::PathBuf::from(fixture.project.to_str().expect("valid UTF-8")),
        piece: None,
        part: "piano".to_owned(),
        sample_rate: RATE,
    })
    .expect("the fixture prepares");
    let mut natively = Vec::with_capacity(frames * 2);
    let mut next = 0usize;
    for frame in 0..frames {
        while next < events.len() && events[next].frame as usize <= frame {
            let event = events[next];
            match event.kind {
                MUSA_AU_EVENT_NOTE_ON => {
                    native.note_on(event.voice, event.data1, event.data2);
                }
                _ => {
                    native.note_off(event.voice, event.data2);
                }
            }
            next += 1;
        }
        let [l, r] = native.step();
        natively.push(l);
        natively.push(r);
    }

    assert_eq!(
        through_abi, natively,
        "the C boundary changed what the prepared instrument sounds like"
    );
}

/// A reset stops every voice, and what is already in the studio decays.
///
/// Reset silences the *instrument*. What the studio has already been sent
/// keeps decaying, because truncating it would be the component overriding
/// the studio the source declared — and it is bounded: the fixture's tail is
/// gone inside a fifth of a second, and nothing re-attacks.
#[test]
fn a_reset_stops_every_voice() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    let sounding = render(instrument, &phrase()[..1], 512, 512);
    assert!(
        sounding.iter().any(|sample| sample.abs() > 1e-6),
        "the fixture instrument should sound"
    );
    unsafe { musa_au_reset(instrument) };
    assert_eq!(unsafe { musa_au_rendered_frames(instrument) }, 0);

    let after = render(instrument, &[], 19_200, 512);
    unsafe { musa_au_instrument_release(instrument) };
    let (decaying, settled) = after.split_at(19_200);
    assert!(
        settled.iter().all(|sample| *sample == 0.0),
        "the instrument was still sounding a fifth of a second after a reset"
    );
    // Nothing re-attacks: each successive tenth of the tail is quieter than
    // the one before it. A voice that survived the reset would break this
    // before the silence above ever failed.
    let peaks: Vec<f32> = decaying
        .chunks(1920)
        .map(|chunk| chunk.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs())))
        .collect();
    for pair in peaks.windows(2) {
        assert!(
            pair[1] <= pair[0],
            "the output grew after a reset — a voice survived it: {peaks:?}"
        );
    }
}

/// Preparation names what it was prepared from.
#[test]
fn a_preparation_states_its_identity() {
    let fixture = Fixture::new();
    let preparation =
        unsafe { musa_au_prepare(fixture.project.as_ptr(), std::ptr::null(), fixture.part.as_ptr(), RATE) };
    let music = unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_identity_music(preparation)) };
    let part = unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_identity_part(preparation)) };
    assert!(
        !music.to_bytes().is_empty(),
        "a prepared instrument has a music identity"
    );
    assert_eq!(part.to_bytes(), b"piano");
    unsafe { musa_au_preparation_release(preparation) };
}
