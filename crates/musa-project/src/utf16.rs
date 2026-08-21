//! The unit source offsets are measured in, on each side of the wire.
//!
//! Rust measures the source in **bytes**. `musa-syntax` is built on Rowan
//! and `text-size`, whose ranges are byte ranges; this crate passes that
//! measure through, so a [`Span`](crate::Span) on a diagnostic, on a fact,
//! and on a [`TextEdit`](crate::TextEdit) are all the same kind of thing, and
//! so `musa` can hand one to `miette` and have it point at the right
//! character. That stays true and is not what this module changes.
//!
//! A JavaScript frontend measures strings in **UTF-16 code units**, and so
//! does `CodeMirror`, whose document positions are what the desktop editor's
//! decorations, lint ranges, and caret are stated in. For ASCII the two
//! measures agree, which is why the difference stayed invisible for a long
//! time. They diverge at the first character above U+007F: an em dash is
//! three bytes and one code unit, so every offset after it is two too large
//! on the JavaScript side — a decoration two characters off, a caret in the
//! middle of the wrong word, a lint underlining the wrong token.
//!
//! So the contract, fixed by `docs/rules/desktop/03-interaction.md` §7, is drawn
//! at the wire rather than inside Rust: **the serialized snapshot states
//! every span in UTF-16 code units, and the Rust API states every span in
//! bytes.** [`ProjectSnapshot::to_wire`](crate::ProjectSnapshot::to_wire) is
//! the only way to produce that snapshot — [`ProjectSnapshot`] deliberately
//! does not implement `Serialize`, so there is no second path that could skip
//! the translation. A caller that sends offsets *back* — a text edit built
//! from a diagnostic the frontend was shown — restates them with
//! [`Utf16Offsets::to_bytes`].

/// Byte offsets ↔ UTF-16 code-unit offsets for one source text.
///
/// The translator behind the wire contract described in the module
/// documentation. Public because the translation is two-way: a frontend that
/// was handed UTF-16 spans and builds a [`TextEdit`](crate::TextEdit) from
/// one has to restate it in bytes before the session can apply it, and this
/// is what it restates it with.
///
/// Built once per translation, in `O(len)`, and answers in `O(log n)` where
/// *n* is the number of characters above U+007F. A source with none — every
/// example in this repository but one — builds two empty vectors and answers
/// by identity, so the common case costs an `is_ascii` scan and nothing else.
#[derive(Debug, Default)]
pub struct Utf16Offsets {
    /// The byte offset just past each non-ASCII character, ascending.
    bytes: Vec<u32>,
    /// The UTF-16 offset of the same position, in the same order.
    units: Vec<u32>,
}

impl Utf16Offsets {
    /// Index `source`.
    pub fn new(source: &str) -> Self {
        if source.is_ascii() {
            return Self::default();
        }
        let mut index = Self::default();
        let mut units: u32 = 0;
        for (offset, character) in source.char_indices() {
            let start = u32::try_from(offset).unwrap_or(u32::MAX);
            if character.is_ascii() {
                units = units.saturating_add(1);
                continue;
            }
            let length = u32::try_from(character.len_utf8()).unwrap_or(1);
            units = units.saturating_add(u32::try_from(character.len_utf16()).unwrap_or(1));
            index.bytes.push(start.saturating_add(length));
            index.units.push(units);
        }
        index
    }

    /// The UTF-16 offset of the byte offset `at`.
    pub fn to_utf16(&self, at: u32) -> u32 {
        translate(&self.bytes, &self.units, at)
    }

    /// The byte offset of the UTF-16 offset `at`.
    pub fn to_bytes(&self, at: u32) -> u32 {
        translate(&self.units, &self.bytes, at)
    }
}

/// Map `at` from the `from` measure to the `to` measure.
///
/// Both vectors name the same character boundaries in their own unit, so an
/// offset is translated by finding the last boundary at or before it and
/// carrying the remaining ASCII distance across unchanged. An offset that is
/// not on a character boundary — which none of the compiler's are, since they
/// all come from the lexer — lands inside the character it fell into rather
/// than panicking.
fn translate(from: &[u32], to: &[u32], at: u32) -> u32 {
    let boundary = from.partition_point(|edge| *edge <= at);
    match boundary.checked_sub(1) {
        None => at,
        Some(previous) => {
            let (Some(from), Some(to)) = (from.get(previous), to.get(previous)) else {
                return at;
            };
            to.saturating_add(at.saturating_sub(*from))
        }
    }
}

/// Restate every span in a serialized snapshot in UTF-16 code units.
///
/// The walk is *structural*, not a list of field names: a span is the only
/// `{ "start": u32, "end": u32 }` object in the wire format, so a span added
/// to the facts is translated the day it appears rather than the day someone
/// remembers to add it here. `wire_laws::every_start_end_object_is_a_source_span`
/// is what holds that claim up: it fails if an object of that shape ever
/// comes to mean something else, at which point the walk has to stop being
/// structural.
pub(crate) fn translate_spans(value: &mut serde_json::Value, offsets: &Utf16Offsets) {
    match *value {
        serde_json::Value::Object(ref mut fields) => {
            if let (2, Some(start), Some(end)) = (fields.len(), span_end(fields, "start"), span_end(fields, "end")) {
                fields.insert("start".to_owned(), offsets.to_utf16(start).into());
                fields.insert("end".to_owned(), offsets.to_utf16(end).into());
                return;
            }
            for field in fields.values_mut() {
                translate_spans(field, offsets);
            }
        }
        serde_json::Value::Array(ref mut items) => {
            for item in items {
                translate_spans(item, offsets);
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

fn span_end(fields: &serde_json::Map<String, serde_json::Value>, name: &str) -> Option<u32> {
    fields.get(name)?.as_u64().and_then(|at| u32::try_from(at).ok())
}

#[cfg(test)]
mod offset_laws {
    use super::{Utf16Offsets, translate_spans};

    /// The reference the index is an acceleration of.
    fn slow(source: &str, at: u32) -> u32 {
        let at = usize::try_from(at).unwrap_or(0);
        let head = source.get(..at).unwrap_or(source);
        u32::try_from(head.encode_utf16().count()).unwrap_or(0)
    }

    #[test]
    fn ascii_is_the_identity() {
        let offsets = Utf16Offsets::new("piece \"twinkle\" { }");
        for at in 0..20u32 {
            assert_eq!(offsets.to_utf16(at), at);
            assert_eq!(offsets.to_bytes(at), at);
        }
    }

    #[test]
    fn every_boundary_agrees_with_counting_the_units() {
        // An em dash (3 bytes, 1 unit), a lone accent (2 bytes, 1 unit), and
        // a character outside the basic plane (4 bytes, 2 units).
        let source = "a — b é c 𝄞 d";
        let offsets = Utf16Offsets::new(source);
        for (at, _) in source.char_indices().chain(std::iter::once((source.len(), ' '))) {
            let at = u32::try_from(at).unwrap_or(0);
            assert_eq!(offsets.to_utf16(at), slow(source, at), "byte {at}");
        }
    }

    #[test]
    fn translation_round_trips_at_every_character_boundary() {
        let source = "% ré — 𝄞\nnote c4/4";
        let offsets = Utf16Offsets::new(source);
        for (at, _) in source.char_indices() {
            let at = u32::try_from(at).unwrap_or(0);
            assert_eq!(offsets.to_bytes(offsets.to_utf16(at)), at, "byte {at}");
        }
    }

    #[test]
    fn a_span_is_translated_wherever_it_is_nested() {
        let offsets = Utf16Offsets::new("% —\nnote");
        let mut wire = serde_json::json!({
            "diagnostics": [{ "message": "x", "span": { "start": 6, "end": 10 } }],
            "score": { "events": [{ "origin": { "span": { "start": 2, "end": 5 } } }] },
        });
        translate_spans(&mut wire, &offsets);
        let at = |pointer: &str| wire.pointer(pointer).cloned().unwrap_or(serde_json::Value::Null);
        assert_eq!(at("/diagnostics/0/span"), serde_json::json!({ "start": 4, "end": 8 }));
        assert_eq!(
            at("/score/events/0/origin/span"),
            serde_json::json!({ "start": 2, "end": 3 })
        );
    }

    #[test]
    fn an_object_that_only_looks_like_a_span_is_left_alone() {
        let offsets = Utf16Offsets::new("—");
        let mut wire = serde_json::json!({ "loop": { "start": 9, "end": 9, "sampleRate": 48_000 } });
        translate_spans(&mut wire, &offsets);
        assert_eq!(wire.pointer("/loop/start"), Some(&serde_json::json!(9)));
    }
}
