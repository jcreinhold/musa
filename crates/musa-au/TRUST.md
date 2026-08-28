# What is trusted in `musa-au`

The workspace denies `unsafe_code`. This crate allows it, at the crate root, with a reason — so this page says exactly
what that buys and what holds it up. Nothing here is trusted because it is convenient; each item is trusted because a C
ABI has no safe spelling of it, and each one is bounded.

## Why the exception exists

`musa-au` is a boundary and not a library. It computes nothing: `musa-project` compiles, verifies, and prepares, and
this crate makes the result reachable from Objective-C. Every `unsafe` here is one of exactly three things — reading a
NUL-terminated argument, turning a host's pointer and length into a slice, or moving a `Box` across the boundary — and
all three are what "a C ABI" *means*.

The alternative is a crate that wraps each of those in a safe helper and marks the helper `unsafe` anyway. That would
satisfy the lint and change nothing about what is trusted, which is the kind of move `AGENTS.md` forbids for the
opposite reason it looks like it would require it.

## The four invariants every entry point keeps

1. **Nothing unwinds into C.** A panic crossing an `extern "C"` frame is undefined behavior. `musa_au_prepare`,
   `musa_au_render`, `musa_au_render_outputs`, and `musa_au_reset` catch; the accessors cannot panic because they only
   read already-built values.
2. **Every pointer is checked for null once, at the boundary.** `as_ref`/`as_mut` do it for handles; `borrow` does it
   for strings. Past that line the code holds references and the ordinary rules apply.
3. **Nothing borrowed escapes.** Every `const char *` handed out is owned by the object that produced it and lives
   exactly as long as that object. The generated header says so beside each accessor, and the Swift side copies before
   releasing.
4. **Layout is asserted on both sides.** `MusaAuEvent` and `MusaAuControl` are `#[repr(C)]`, the generated header
   carries `_Static_assert`s for each one's size and every field offset, and a currency test regenerates the header and
   compares it byte for byte. A layout disagreement is a compile error on one side or a test failure on the other, never
   a rendered artifact.

## What the caller must guarantee

Stated once in `lib.rs` and per function where it adds something:

- a pointer is null, or came from this library and has not been released;
- a string pointer is NUL-terminated UTF-8;
- `events` addresses `count` initialized `MusaAuEvent` values, or is null when `count` is zero;
- `left` and `right` each address `frames` writable floats and do not alias;
- `channels` addresses `channel_count` non-null pointers, each to `frames` writable floats, none aliasing another;
- one handle is not used from two threads at once.

## The one place a pointer is written through more than once

`musa_au_render_outputs` is the only entry point that writes through a caller-supplied *array* of pointers rather than
two named ones, so it is the only one where "none of them alias" is a promise about a set. Three things bound it. The
channel pointers are copied into a fixed stack array whose length is `MUSA_AU_MAX_OUTPUTS * 2`, so a `channel_count`
larger than the ABI's own maximum is refused rather than read; an odd `channel_count` and any null channel are refused
outright, because a caller that has not said which channel it means must not have one guessed for it; and the writing
itself goes through one private `write_frame`, which indexes the copied array with `get` and returns rather than writing
when an index is not there. `tests/suite/controls.rs` renders a partition of the same frames into one, two, and three
outputs and compares bus zero, which is the observable half of the same claim.

`tests/suite/boundary.rs` exercises the negative half of each of these that can be exercised from Rust — null handles,
null strings, non-UTF-8 strings, a zero-length block, a null event pointer with a nonzero count, a taken preparation
taken again, and a released handle's accessors. Aliasing and cross-thread use are stated obligations rather than tested
ones, because a test that violated them would be testing undefined behavior.

## What is *not* trusted

Everything musical. This crate never decides a pitch, a duration, a control value, or whether a source is valid; it
forwards. A bug in what an instrument sounds like is a bug in `musa-project` or below, and this page is not where to
look for it.
