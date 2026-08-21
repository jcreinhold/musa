---
id: 54
slug: editable-facts
status: done
depends_on: [26, 51]
phase: 2
---

# Editable Where It Is Printed

## Task

Every fact the interface prints *about the piece* — its title, its front matter, its tempo, key, and meter — becomes
editable in the place it is printed, and becomes visibly editable at rest. The app currently prints nine such facts and
lets a composer change none of them; the two things it does let them change, a note's pitch and its duration, announce
themselves only to someone whose pointer happens to already be on them. Someone who has not read the source cannot tell
that anything on this screen is a control.

## Read

- `docs/rules/desktop/00-thesis.md` §2 — "a field is a value with a hairline underline, not a bordered input". This
  prompt keeps the rule and repairs *when the underline is drawn*.
- `docs/rules/desktop/01-visual-language.md` §7 (the inspector's rows and the hover-only affordance), §2 (two hues —
  this adds none).
- `docs/rules/desktop/03-interaction.md` §2 (the pointer table), §7 (the exhaustive list of what the frontend may
  compute; this prompt stays inside it, and the design says how).
- `docs/rules/desktop/05-states.md` §2 — an empty screen is an invitation to act. The inspector's `Nothing selected.` is
  the counter-example this prompt deletes.
- Roadmap §14.6 (every edit is a `ProjectCommand` resolved to text in Rust), §2 (the layer table).
- Prompt 51's `FrontMatter` and its `<pgFoot>` precedent; prompt 26's `EditableValue` and `TypographicRow`.

## Design

### The one idea

**Nothing selected is not nothing.** With no note selected, the inspector shows *the piece* — every statement its header
can carry, including the ones it does not carry yet. One move answers three questions: what can I change here, what has
this piece already said, and where do I click to say something it has not.

That is also the whole answer to "how would anyone know they can annotate the composer". Not a help bar, not a tour, not
a first-run overlay: **the list of what a piece can say is the list of fields, and the empty ones are on it.** A help
bar would be a second place to state something the interface should be able to show directly, and it would need to be
maintained against the thing it describes.

### 1. Editable is visible at rest

An editable value carries a hairline underline in three weights:

| State | The underline |
| --- | --- |
| At rest | `1px` `--rule` |
| Hover | `1px` `--ink-muted` |
| Focus | `1px` `--plate` |

Only editable values get one, which is what makes it a signal and not decoration — `POSITION`, `VOICE`, and `ORIGIN`
stay bare, and the difference between the rows that take a value and the rows that report one becomes visible without
reading either.

This is a repair, and the reason is worth stating: an affordance that appears on hover can only be discovered by someone
who already suspects it is there. The thesis's actual claim — no bordered inputs, no button chrome, the value *is* the
control — survives intact; a hairline at rest is not a box.

### 2. The piece, in the inspector

With nothing selected, the inspector is a run of `TypographicRow`s in the source's own order:

```
TITLE       Glass Mountain
SUBTITLE    for violin and strings
COMPOSER    musa
ARRANGER    —
COPYRIGHT   © 2026. Licensed CC BY-SA 4.0.
TEMPO       quarter = 72
KEY         a minor
METER       4/4
```

- **A role the piece has not named still gets a row**, with an em dash in `--ink-muted` where the value goes. The field
  is there, it is empty, and it takes a value. This is the discovery surface.
- **"Nothing selected" had to be made to exist first.** `Workspace.focused` falls back to the score's first event so
  that entry, the active voice, and the position readout always have somewhere to start — which meant the inspector was
  never handed `undefined` and its empty branch was unreachable code. The two questions are different: `focused` is
  "where is work happening", `chosen` is "what did the composer pick", and only the second is allowed to have no answer.
  The inspector reads `chosen`. Keyboard navigation still starts from `focused`, so the first arrow on a freshly opened
  piece still lands in the music.
- **Committing an empty value into a named role deletes the statement.** Adding and removing a line of front matter are
  the same gesture, and neither needs a button.
- **The title cannot be emptied.** A piece with no name has nothing for the file, the page head, or the frame to print;
  the field refuses and says so in the core's own words, like any other refusal.
- **Values are spelled the way the source spells them** — `quarter = 72`, `a minor`, `4/4`. The band may go on printing
  `♩ = 72` and `A minor` at rest, but what a composer types is musa, because a field that accepted a second dialect
  would be a second language to keep working.

### 3. Editable on the page

Clicking the title, subtitle, composer, arranger, or copyright *on the engraved page* puts an input over it, in the
page's own face and at its own size, and commits on `Return` or on blur — the same commit rule `EditableValue` already
uses everywhere else. `Esc` puts it back. Nothing else on the page becomes editable.

This needs no new mechanism, because **Verovio passes an encoded page head's `xml:id`s straight through to the SVG**,
exactly as it does a note's — measured, not assumed. So the MEI backend writes the head itself, with `front-title`,
`front-subtitle`, `front-composer`, and `front-arranger` ids, `<pgFoot>`'s line gains `front-copyright`, the app asks
for `header: "encoded"` in place of `"auto"`, and hit-testing a title is then the same machinery, and the same clause of
§7, as hit-testing a notehead.

**On the page the underline is hover-only, and this is the one exception to §1.** Two reasons, and the second is the one
that decides it. The chrome is an interface and can afford to advertise; the page is the artifact, and a title
permanently underlined is a page that looks like a web form rather than like an edition — the opposite of what the page
apparatus was added for. Discovery does not depend on it, because §2's list prints all five roles whether or not the
piece has filled them in. And the mechanism §1 uses is not available here anyway: Blink paints `text-decoration` on SVG
text with the glyph's own fill, so `text-decoration-color: transparent` does nothing and a rest state declared that way
would underline the whole head. The hover hairline is therefore *drawn* — a `1px` `--ink-muted` rule positioned over the
page from the line's measured box, which is the same thing a selection halo is and lands in the same clause of §7.

**What this costs, stated plainly.** Prompt 51 promised musa would never say *where* a line of front matter sits, and an
encoded head says `halign="center"` for the title and `halign="right"` for the composer. Prompt 51 had in fact already
crossed that line for the copyright, and had given the reason: `<pgFoot>` is MEI's own way of saying "this line belongs
at the foot". So the boundary is not "musa never speaks about place". It is:

> musa may use MEI's own vocabulary for **which region of the page a line of front matter belongs to** — head or foot,
> centred or right. It may not state a coordinate, a margin, a rastral size, a system or page break, or anything at all
> that is per-page.

Both prompt 51 and `02-engraving.md` §4 are repaired to say that once, in those words. If it cannot be written down that
plainly then the encoded head is the wrong trade and this section should be cut back to the inspector and the band.

A consequence to carry: `header: "auto"` was also drawing the running head on pages after the first. The encoded head
must supply `<pgHead2>` for that, or the running head is lost — check it against a fixture that actually runs to two
pages before claiming otherwise.

### 4. The band

`TEMPO`, `KEY`, and `METER` in the top band are the same fields as the inspector's, in a second place. They get the same
underline and the same commit rule. A composer looking at the band is looking at the piece's tempo; making them find the
inspector to change it is the app knowing something it will not act on.

### 5. Rust: one edit

```rust
EditCommand::SetHeader { field: HeaderField, value: String }
```

with `HeaderField` ∈ {`Title`, `Subtitle`, `Composer`, `Arranger`, `Copyright`, `Tempo`, `Meter`, `Key`}. Resolved in
`musa-project` against the CST, like every other structured edit:

- a statement the piece has → replace its value tokens, and nothing else on the line;
- a statement it does not have → insert it, in the order the formatter would have put it;
- an empty value → delete the statement, whitespace and all (`Title` refuses instead).

Transactional like every other edit (roadmap §14.6): a value that does not parse leaves the session unchanged and comes
back as the compiler's own diagnostic. **The frontend validates nothing** — not the tempo's number, not the key's mode,
not the meter's denominator. That judgement lives in one place and this is not it.

## Target

- `musa-project`: `HeaderField`, `EditCommand::SetHeader`, its resolution and its refusals; the wire shape follows the
  other edits.
- `musa-notation`: the encoded `<pgHead>` with front-matter ids; `front-copyright` on the `<pgFoot>` line.
- `apps/musa-desktop/ui`:
  - `header: "encoded"`, and `<pgHead2>` proven or the loss reported.
  - `TypographicRow` / `EditableValue` — the rest underline.
  - `Inspector.svelte` — the piece view where `Nothing selected.` was.
  - `TransportReadout.svelte` — tempo, key, meter as fields.
  - `lib/score/` — front-matter hit-testing and the in-place input over the page.
- Tests:
  - Rust: each field set on a piece that has it and a piece that does not; each field emptied; `Title` refusing; a value
    that does not compile leaving the session unchanged; the inserted statement landing where the formatter would put
    it.
  - unit: the id → field mapping.
  - Playwright: type into the empty inspector row, click the title on the page and rename the piece, change the meter in
    the band — and assert in each case the `setHeader` the interface *issued*, plus that the rename is a single undo.
    What the edit then does to the text is asserted in Rust and not here: the screen tests run against a stub, so a
    Playwright assertion about the source or about a diagnostic would be an assertion about the stub. The refusals the
    Target's Rust bullet lists are the same list, tested where the judgement actually lives.
  - Playwright: hovering a line of front matter draws the hairline and hovering a note does not.
  - a golden of the inspector's piece view and of the page mid-rename, at 1440, both themes.
- Docs: the boundary sentence into `51-engraved-edition.md` and `02-engraving.md` §4; the rest-underline into
  `00-thesis.md` §2 and `01-visual-language.md` §7; the page-field row into `03-interaction.md` §2.

## Check

```sh
cargo nextest run -p musa-project -p musa-notation
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
```

Commit as `Make the piece's own facts editable`.

## Stop

- No new hue, no boxes, no buttons. The underline is the entire affordance.
- No help bar, no guided tour, no first-run overlay, no tooltips or popovers.
- Nothing on the page becomes editable except the five front-matter lines — notes are prompt 53, and part names,
  dynamics, and tempo marks are neither.
- No metadata beyond prompt 51's four roles.
- The inspector's piece view is statements the source could hold. It does not become a settings panel, and nothing about
  the *application* appears in it.
