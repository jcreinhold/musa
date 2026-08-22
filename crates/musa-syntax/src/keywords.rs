//! Every keyword's own plain-English documentation.
//!
//! The lexer owns the spellings; this table owns what they mean, so the two
//! cannot drift apart silently: [`keyword_doc`] is a wildcard-free match over
//! [`SyntaxKind`], and the workspace forbids wildcard match arms, so a keyword
//! added to the lexer without a doc arrives as a compile error. That is the
//! same discipline `Code` and `musa_project::explain` keep for diagnostics.
//!
//! Doc style: what the construct *is*, then how or when to use it, in two
//! sentences at most, plus one `musa` example. A keyword a composer cannot
//! hover is a keyword the language has not finished explaining.

use crate::SyntaxKind;

/// One keyword's documentation.
pub struct KeywordDoc {
    /// How the keyword is written.
    pub spelling: &'static str,
    /// One clause, for lists and tooltips.
    pub summary: &'static str,
    /// Plain English: what the construct is, how to use it, one `musa`
    /// example.
    pub doc: &'static str,
}

macro_rules! doc {
    ($spelling:literal, $summary:literal, $doc:literal) => {
        KeywordDoc {
            spelling: $spelling,
            summary: $summary,
            doc: $doc,
        }
    };
}

static PIECE: KeywordDoc = doc!(
    "piece",
    "one file's one composition",
    "A piece is one composition: everything a single file says about one work — its title, its starting tempo, \
     meter, and key, its score, and its studio. Every `.musa` file that is not a `library` holds exactly one.\n\n\
     ```musa\npiece \"Ruchenitsa\" { meter 7/8; score { … } }\n```"
);
static LIBRARY: KeywordDoc = doc!(
    "library",
    "declarations other pieces can import",
    "A library is a file of declarations — motifs, fragments, studio patches — meant to be used by other files \
     rather than played itself. A piece brings one in with `import \"path\";` and then speaks its names as its own.\n\n\
     ```musa\nlibrary { motif sigh(root: Pitch) { … } }\n```"
);
static TEMPO: KeywordDoc = doc!(
    "tempo",
    "how fast, written where it changes",
    "A tempo statement is a speed: a metronome mark, a word, or both, and a change of speed is a new statement \
     written where the music reaches it — not a property of the piece. With `to` and `over` it is a gradual \
     change that names where it arrives and how far that takes.\n\n\
     ```musa\ntempo 1/4 = 96;\ntempo 1/4 = 72 to 48 over 2/1 \"rit.\";\n```"
);
static METER: KeywordDoc = doc!(
    "meter",
    "the count, written where it changes",
    "A meter statement is the count the music is measured in, as a fraction of the whole note: `meter 7/8;` \
     counts seven eighths to a bar. It changes mid-piece by being written again where it changes, and \
     `meter none;` stops the barlines entirely until the next one.\n\n\
     ```musa\nmeter 4/4;\n```"
);
static KEY: KeywordDoc = doc!(
    "key",
    "the key signature, written where it changes",
    "A key statement names the key the music is in — a pitch class and a mode. It is notation and knowledge, \
     not a transposition: the notes are written at their sounding pitch either way.\n\n\
     ```musa\nkey a minor;\n```"
);
static SUBTITLE: KeywordDoc = doc!(
    "subtitle",
    "a second line under the title",
    "A subtitle is a line of front matter printed under the piece's title — a dedication, a movement name, an \
     opus number.\n\n\
     ```musa\nsubtitle \"for eight players\";\n```"
);
static COMPOSER: KeywordDoc = doc!(
    "composer",
    "who wrote it",
    "The composer line names who wrote the piece, printed in the front matter.\n\n\
     ```musa\ncomposer \"J. S. Bach\";\n```"
);
static ARRANGER: KeywordDoc = doc!(
    "arranger",
    "who arranged it",
    "The arranger line names who arranged the piece, printed in the front matter beside the composer.\n\n\
     ```musa\narranger \"F. Busoni\";\n```"
);
static COPYRIGHT: KeywordDoc = doc!(
    "copyright",
    "the rights line",
    "The copyright line is the rights notice printed in the front matter.\n\n\
     ```musa\ncopyright \"© 2026 J. Reinhold\";\n```"
);
static MOTIF: KeywordDoc = doc!(
    "motif",
    "a named musical idea, used by name",
    "A motif is a musical idea said once and given a name: a phrase, a figure, a rhythm, with parameters for \
     what changes between uses. Write it out where it is wanted with `use name(args);` — an edit to the motif \
     then reaches every occurrence.\n\n\
     ```musa\nmotif sigh(root: Pitch) { root/8 root/8 }\n```"
);
static FRAGMENT: KeywordDoc = doc!(
    "fragment",
    "a labeled block a mobile can arrange",
    "A fragment is a named block of music that means nothing in source order: it becomes music when a `mobile` \
     arranges it among others. Fragments are how musa writes music whose order is chosen, not fixed.\n\n\
     ```musa\nfragment a { c4/4 d4/4 }\n```"
);
static SCORE: KeywordDoc = doc!(
    "score",
    "the parts and their music",
    "The score block is the music itself: the parts, each with its clef and its voices, written in the order \
     they read. Everything before it is the header; everything after it is the studio.\n\n\
     ```musa\nscore { part violin { clef treble; voice one { … } } }\n```"
);
static PART: KeywordDoc = doc!(
    "part",
    "one instrument's staff",
    "A part is one instrument's lane through the piece: its own clef, its own voices, and — when it differs \
     from the piece — its own meter or tempo (polymeter, polytempo). A part is notation, not sound; the studio \
     decides what plays it.\n\n\
     ```musa\npart kaval { clef treble; meter 7/8; voice melody { … } }\n```"
);
static VOICE: KeywordDoc = doc!(
    "voice",
    "one rhythmic line inside a part",
    "A voice is one line of rhythm inside a part: notes, rests, and everything written where the music reaches \
     it. Two voices in one part share the staff and count their own bars independently.\n\n\
     ```musa\nvoice right { c4/4 d4/4 }\n```"
);
static CLEF: KeywordDoc = doc!(
    "clef",
    "which clef the staff reads",
    "A clef statement says which clef a part reads from here — `treble`, `bass`, `alto`, `tenor`. Written in a \
     voice, it changes mid-part where the music reaches it.\n\n\
     ```musa\nclef treble;\n```"
);
static PROFILE: KeywordDoc = doc!(
    "profile",
    "which performance profile realizes this part",
    "A profile statement names the performance profile a part is played with — a named bundle of humanization \
     settings declared in a `performance` block. Without one, the part is played as written.\n\n\
     ```musa\nprofile strings;\n```"
);
static PERFORMANCE: KeywordDoc = doc!(
    "performance",
    "rules for how the written becomes the played",
    "A performance block declares how notation becomes playing: `mark`, `dynamic`, `groove`, and `grace` rules \
     that say what a marking does to time and loudness. It is the piece's own answer to \"what does this mark \
     mean here\", kept separate from the notation it interprets.\n\n\
     ```musa\nperformance { mark accent { weight: 1.3; } }\n```"
);
static USE: KeywordDoc = doc!(
    "use",
    "write out a motif where it stands",
    "`use` writes out a declared motif where it stands, with arguments for its parameters: `use sigh(e5);` is \
     the motif spelled here, once. It is a splice and not a call: the music arrives at the cursor and is \
     sequenced there. Bringing in another file is `import`, which is a different statement and now a different \
     word.\n\n\
     ```musa\nuse sigh(e5);\n```"
);
static IMPORT: KeywordDoc = doc!(
    "import",
    "bring another file's names into this one",
    "`import` reads a relative library file or a bundled module and binds its declarations as if they were \
     written here: `import \"../library/motifs.musa\";` or `import std::core;`. Add `as name` to bring it in \
     under a different name. Bundled modules are ordinary Musa source and add no hidden prelude.\n\n\
     ```musa\nimport std::core;\n```"
);
static SYNTAX: KeywordDoc = doc!(
    "syntax",
    "the region a package reads, and the import that names its reader",
    "`syntax` writes two things, and both name an adapter: `import syntax std::adapters::doubled as doubled;` \
     in the header says which package reads a region, and `syntax doubled { ... }` is the region it reads. \
     The contents are not ordinary Musa — the adapter turns them into an expression, and everything after \
     that is ordinary resolution and type checking.\n\n\
     ```musa\nimport syntax std::adapters::doubled as doubled;\nlet total = syntax doubled { 3 };\n```"
);
static TRANSPOSE: KeywordDoc = doc!(
    "transpose",
    "the same music, moved in pitch",
    "Transpose plays music moved by a written interval such as `P5` or `M3`; the block and function forms have \
     the same musical meaning. Supplying only the interval makes a reusable answer function.\n\n\
     ```musa\nlet answer: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = transpose(P5);\n```"
);
static UP: KeywordDoc = doc!(
    "up",
    "the direction of a transposition, upward",
    "`up` marks a transposition as upward: `transpose up P5 { … }` raises its contents a perfect fifth. Its \
     counterpart is `down`.\n\n\
     ```musa\ntranspose up M3 { … }\n```"
);
static DOWN: KeywordDoc = doc!(
    "down",
    "the direction of a transposition, downward",
    "`down` marks a transposition as downward: `transpose down P5 { … }` lowers its contents a perfect fifth. \
     Its counterpart is `up`.\n\n\
     ```musa\ntranspose down m2 { … }\n```"
);
static REST: KeywordDoc = doc!(
    "rest",
    "silence with a duration",
    "A rest is notated silence: it fills its duration in the bar exactly like a note fills its own. A bar that \
     does not add up — rests included — is an error, because a missing rest is a missing beat.\n\n\
     ```musa\nrest/4\n```"
);
static REPEAT: KeywordDoc = doc!(
    "repeat",
    "repeat music",
    "A repeat block plays its contents more than once: `repeat 4 { … }` is four passes through the same bars, \
     written once. With `ending` blocks inside, the passes differ where the endings say they do. The word is a \
     statement keyword and nothing else: the finite value operation that makes a list of `count` copies is \
     `repeated(value, count)` in `std::list`, spelled apart because `fn repeat` does not parse.\n\n\
     ```musa\nrepeat 2 { use verse(); }\nlet pedals: List<Bool> = repeated(true, 4);\n```"
);
static ASSERT: KeywordDoc = doc!(
    "assert",
    "a claim the compiler proves",
    "An assertion states something objective about the music inside it and makes the compiler check it: \
     `assert pitches_in(scale c major) { … }` says every note sounded there is spelled in C major, and a note \
     that is not stops the compilation with the note named. It changes nothing — a claim that holds returns \
     exactly what was written — and it is per-passage, never a rule about the piece.\n\n\
     The claims are `fills_meter()`, `pitches_in(scale)`, `realizes(chord, policy)`, `voices(count)`, and \
     `within_ranges(ranges)`.\n\n\
     ```musa\nassert voices(4) { [c3 g3 e4 c5]/1 }\n```"
);
static BAR: KeywordDoc = doc!(
    "bar",
    "one measure, named",
    "A bar is one measure of music: what a player reads between two barlines, and the unit that must add up to \
     the meter. Write it with `|`, the way notation draws it. The keyword is for the bar that earns a name — \
     `bar head { … }` — which makes it an address an edit or a reader can point at.\n\n\
     ```musa\nbar head { c4/4 d4/4 e4/4 f4/4 }\n```"
);
static ENDING: KeywordDoc = doc!(
    "ending",
    "what changes on a given pass of a repeat",
    "An ending block plays only on the numbered pass of the enclosing `repeat`: `ending 1 { … }` the first \
     time, `ending 2 { … }` the second. It is how a repeated strain gets a different close.\n\n\
     ```musa\nrepeat 2 { use strain(); ending 1 { c4/1 } ending 2 { g4/1 } }\n```"
);
static SLUR: KeywordDoc = doc!(
    "slur",
    "a phrase mark over the block",
    "A slur block is played legato under one bow or breath: the notation curve over its contents. It changes \
     how the notes connect, not which notes they are.\n\n\
     ```musa\nslur { c4/8 d4/8 e4/8 }\n```"
);
static PHRASE: KeywordDoc = doc!(
    "phrase",
    "a named musical sentence",
    "A phrase block names a stretch of music as one sentence: `phrase \"A\" { … }`. The name prints and \
     anchors, so editors and readers can talk about the phrase as a thing.\n\n\
     ```musa\nphrase \"question\" { use ask(); }\n```"
);
static MARK: KeywordDoc = doc!(
    "mark",
    "a performance mark: accent, fermata, rehearsal letter, sample",
    "A mark statement attaches a performance mark — `accent`, `fermata`, `breath`, `rehearsal \"A\"`, and more — \
     to the music at it. What a mark *does* to the playing is the `performance` block's `mark` rule, not this \
     statement's: notation here, meaning there.\n\n\
     ```musa\nmark fermata;\nmark rehearsal \"A\";\n```"
);
static DYNAMIC: KeywordDoc = doc!(
    "dynamic",
    "a loudness marking",
    "A dynamic statement is a loudness marking — `pp` through `fff` — written where it takes force. It is a \
     mark on the page, not a decibel value; what it does to the notes is the performance layer's business.\n\n\
     ```musa\ndynamic mf;\n```"
);
static GROOVE: KeywordDoc = doc!(
    "groove",
    "a named timing feel",
    "A groove statement applies a named timing feel — a swing or shuffle declared in the `performance` block — \
     to the music from here. The name is notation; the rule is what it does to the beats.\n\n\
     ```musa\ngroove shuffle;\n```"
);
static GRACE: KeywordDoc = doc!(
    "grace",
    "notes crushed before the beat",
    "A grace block holds grace notes: small notes that lean on the event after them and fill no time in the \
     bar. Where they steal their time from is the `performance` block's `grace` rule.\n\n\
     ```musa\ngrace { d5 } c5/4\n```"
);
static TUPLET: KeywordDoc = doc!(
    "tuplet",
    "notes in the time of fewer",
    "A tuplet block fits its contents into the time a different count would take: `tuplet 3/2 { … }` is three \
     in the time of two. The ratio is exact — tuplets are arithmetic, not feel.\n\n\
     ```musa\ntuplet 3/2 { c4/8 d4/8 e4/8 }\n```"
);
static STRETCH: KeywordDoc = doc!(
    "stretch",
    "the same music, at a different speed ratio",
    "Stretch plays music scaled in written time by an exact factor: `stretch 3/2 { … }` takes half again as \
     long. `stretch(3/2)` is the reusable function form with the same meaning.\n\n\
     ```musa\nlet broaden: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = stretch(3/2);\n```"
);
static RETROGRADE: KeywordDoc = doc!(
    "retrograde",
    "the block, backwards",
    "Retrograde plays its music backwards — the last event first. The block and `retrograde(subject)` function \
     forms are the same classical transformation.\n\n\
     ```musa\nlet answer: EventTrack<WrittenTime> = retrograde(subject);\n```"
);
static INVERT: KeywordDoc = doc!(
    "invert",
    "the block, mirrored in pitch",
    "Invert mirrors music around an axis pitch: what went up goes down by the same written interval. \
     `invert(c5)` makes a reusable function; the block writes the axis with `around`.\n\n\
     ```musa\nlet mirror: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = invert(c5);\n```"
);

static SHIFT_FUNCTION: KeywordDoc = doc!(
    "shift",
    "the same music, entering later",
    "`shift` delays music by an exact written duration without adding a rest event. Supply only the delay to make \
     a reusable entrance function.\n\n```musa\nlet later: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = shift(duration_of(1/2));\n```"
);
static TOGETHER_FUNCTION: KeywordDoc = doc!(
    "together",
    "music sounding together",
    "`together` starts two music values at the same instant and lasts until the later one ends. It expresses \
     simultaneity, not voice or mixer-track identity.\n\n```musa\nuse together(subject, answer);\n```"
);
static MAP_NOTE_PITCHES_FUNCTION: KeywordDoc = doc!(
    "map_note_pitches",
    "replace each sounding written pitch",
    "`map_note_pitches` applies a checked `pitch -> pitch` function to notes and sounded chord notes while keeping \
     their rhythm, marks, spelling provenance, and score context. Key signatures and harmony annotations are not \
     rewritten.\n\n```musa\nuse map_note_pitches(answer_pitch, subject);\n```"
);

/// Documentation for compiler-owned functions spelled as identifiers.
pub fn builtin_doc(name: &str) -> Option<&'static KeywordDoc> {
    match name {
        "shift" => Some(&SHIFT_FUNCTION),
        "together" => Some(&TOGETHER_FUNCTION),
        "map_note_pitches" => Some(&MAP_NOTE_PITCHES_FUNCTION),
        _ => None,
    }
}
static AROUND: KeywordDoc = doc!(
    "around",
    "the axis of an inversion",
    "`around` names the axis pitch an `invert` block mirrors about: `invert around c5 { … }` leaves `c5` where \
     it is and flips everything else across it.\n\n\
     ```musa\ninvert around g4 { … }\n```"
);
static WITH: KeywordDoc = doc!(
    "with",
    "respell one note of a motif occurrence",
    "A `with` clause overrides one parameter of a `use` for this occurrence only: `use sigh() with (root: e5)` \
     is the motif, except here. The definition is untouched.\n\n\
     ```musa\nuse sigh() with (root: e5);\n```"
);
static NOTE: KeywordDoc = doc!(
    "note",
    "a literal note inside an override",
    "`note` introduces a literal note where an override needs one — the spelled-out exception inside a `with` \
     clause.\n\n\
     ```musa\nuse theme() with (pickup: note c5/8);\n```"
);
static SECTION: KeywordDoc = doc!(
    "section",
    "a form marker in the score",
    "A section statement marks a point of form — `section \"B\";` — anchored to the time it is written at, \
     whether or not a note starts there. It prints and navigates; it changes no note.\n\n\
     ```musa\nsection \"Trio\";\n```"
);
static HARMONY: KeywordDoc = doc!(
    "harmony",
    "the chord-symbol lane",
    "A harmony block declares the piece's chord-symbol lane: symbols written at the times they govern, like \
     `harmony { \"Dm7\" 1/1 \"G7\" 1/1 }`. The symbols are parsed, so later tooling reads structure instead \
     of letters.\n\n\
     ```musa\nharmony { \"Cmaj7\" 2/1 }\n```"
);
static CRESCENDO: KeywordDoc = doc!(
    "crescendo",
    "grow toward a named dynamic",
    "A crescendo block grows the loudness of its contents toward a named arrival: `crescendo to f { … }`. The \
     destination is part of the spelling, because a hairpin that arrives nowhere is a hairpin that lies.\n\n\
     ```musa\ncrescendo to f { use build(); }\n```"
);
static DIMINUENDO: KeywordDoc = doc!(
    "diminuendo",
    "fade toward a named dynamic",
    "A diminuendo block fades the loudness of its contents toward a named arrival: `diminuendo to p { … }`. \
     Like the crescendo, the destination is written, not implied.\n\n\
     ```musa\ndiminuendo to pp { use fall(); }\n```"
);
static TO: KeywordDoc = doc!(
    "to",
    "where a change arrives",
    "`to` names where a change arrives: the dynamic a `crescendo` or `diminuendo` reaches, or the tempo a \
     gradual `tempo` change ends at. Arriving somewhere is one idea, so it is one word.\n\n\
     ```musa\ntempo 1/4 = 72 to 48 over 2/1;\n```"
);
static OVER: KeywordDoc = doc!(
    "over",
    "how far a gradual change reaches",
    "`over` says how far a gradual change reaches, in the change's own unit: `tempo 1/4 = 72 to 48 over 2/1;` \
     takes two whole notes to get there. It can also follow a worded marking — `tempo \"rit.\" over 1/1;` — \
     where the player owns the speed.\n\n\
     ```musa\ntempo \"poco rit.\" over 1/1;\n```"
);
static SENZA: KeywordDoc = doc!(
    "senza",
    "an unmeasured stretch with a scope",
    "A senza block is unmeasured music with braces around it: the barlines stop for exactly its contents and \
     the meter returns after, without the `meter none;` … `meter 4/4;` pair you could otherwise forget the \
     second half of. Cadenzas and chant are written this way.\n\n\
     ```musa\nsenza { c4/1 g4/1 }\n```"
);
static MOBILE: KeywordDoc = doc!(
    "mobile",
    "fragments in a chosen order",
    "A mobile block plays its fragments in an order the performance chooses: `mobile { a; b; c; }` is those \
     fragments, balanced like the sculpture, not fixed like a score. Each performance records the order it \
     picked, so the chosen form is a fact you can read.\n\n\
     ```musa\nmobile { a; b; a; c; }\n```"
);
static IMPROVISE: KeywordDoc = doc!(
    "improvise",
    "a constrained improvisation",
    "An improvise statement asks for an improvisation over a span and a harmony: `improvise 8/1 over \"Dm7 | \
     G7\";`. The constraints are written; the notes are the performance's, and the seed decides which ones.\n\n\
     ```musa\nimprovise 4/1 over \"Cmaj7\";\n```"
);
static STUDIO: KeywordDoc = doc!(
    "studio",
    "what plays the score",
    "The studio block is everything after the music: the patches that make sound, the assignments of parts to \
     them, the buses, sends, routes, and modulations that wire the mix. The score says what the notes are; the \
     studio says what they sound like.\n\n\
     ```musa\nstudio { patch pad { … } assign violin -> pad; }\n```"
);
static PATCH: KeywordDoc = doc!(
    "patch",
    "a signal graph with a name",
    "A patch is a named signal chain: oscillators, filters, envelopes, and effects wired with `|>` into one \
     instrument. A part is realized by the patch an `assign` points at it — and a patch nothing assigns is \
     wired to silence.\n\n\
     ```musa\npatch glass_pad { oscillator(sine) |> lowpass(cutoff: 1400 Hz) |> output; }\n```"
);
static BUS: KeywordDoc = doc!(
    "bus",
    "a shared channel in the mix",
    "A bus is a named channel signals can be sent to and mixed through — a reverb hall every part shares. Parts \
     reach it with `send`; it reaches the output with `route`.\n\n\
     ```musa\nbus hall { reverb(room: 0.8); }\n```"
);
static ASSIGN: KeywordDoc = doc!(
    "assign",
    "which patch realizes a part",
    "An assign statement points a part at a patch: `assign violin -> glass_pad;` says the violin's notes are \
     played by that graph. A part without one keeps the default instrument.\n\n\
     ```musa\nassign reeds -> reed;\n```"
);
static ROUTE: KeywordDoc = doc!(
    "route",
    "where a signal goes in the mix",
    "A route statement sends a part's or bus's output to a destination: `route violin -> master;`. Without \
     one, a part still reaches the master; a route is for when it should go somewhere else, or somewhere \
     else too.\n\n\
     ```musa\nroute hall -> master;\n```"
);
static SEND: KeywordDoc = doc!(
    "send",
    "tap some of a signal into a bus",
    "A send statement taps a level of a part's or bus's signal into a bus: `send violin -> hall at -18 dB;` \
     puts some of the violin in the hall while it also plays dry. The level is written with `at`.\n\n\
     ```musa\nsend strings -> hall at -14 dB;\n```"
);
static MODULATE: KeywordDoc = doc!(
    "modulate",
    "let a signal drive a parameter",
    "A modulate statement wires a control signal into a patch parameter: `modulate lfo -> \
     glass_pad.lowpass.cutoff;` lets the LFO move the filter. The path names patch, node, and parameter with \
     dots.\n\n\
     ```musa\nmodulate lfo -> pad.lowpass.cutoff;\n```"
);
static MASTER: KeywordDoc = doc!(
    "master",
    "the final mix bus",
    "`master` is the mix's final destination: the bus everything audible ultimately routes to. Route to it \
     explicitly when a signal should arrive from more than one path.\n\n\
     ```musa\nroute hall -> master;\n```"
);
static AT: KeywordDoc = doc!(
    "at",
    "the level of a send",
    "`at` writes the level of a `send`: `send violin -> hall at -18 dB;` is how much of the violin the hall \
     gets, in decibels.\n\n\
     ```musa\nsend drums -> room at -24 dB;\n```"
);
static OUTPUT: KeywordDoc = doc!(
    "output",
    "where a patch's signal leaves",
    "`output` ends a patch's signal chain: whatever reaches it is the sound the patch makes. A chain without \
     one builds to nothing, and the compiler says so.\n\n\
     ```musa\noscillator(sine) |> gain(-15 dB) |> output;\n```"
);
static PITCH_KW: KeywordDoc = doc!(
    "pitch",
    "the standard library's pitch module",
    "`pitch` names a module of the standard library — `import std::pitch;` brings its written-pitch and interval \
     operations into scope. It is no longer the type: a type is spelled with a capital, so a motif parameter is \
     written `root: Pitch`.\n\n\
     ```musa\nimport std::pitch;\n\nmotif call(root: Pitch = c5) { root/4 }\n```"
);
static LET: KeywordDoc = doc!(
    "let",
    "give a typed value a name",
    "A `let` declaration gives an elaboration value a name. The annotation keeps the musical domain visible at the declaration site.\n\n\
     ```musa\nlet answer: Interval = P5;\n```"
);
static FN: KeywordDoc = doc!(
    "fn",
    "define a total named function",
    "A function computes an elaboration value from typed parameters. Musa functions are total: they have no unrestricted recursion or effects.\n\n\
     ```musa\nfn identity(x: Pitch) -> Pitch { x }\n```"
);
static MUSIC: KeywordDoc = doc!(
    "music",
    "a notation-first music value",
    "A `music` block is an expression whose body reads like an ordinary voice: notes remain self-delimiting and reusable material is written with `use`.\n\n\
     ```musa\nlet call: EventTrack<WrittenTime> = music { c5/4 d5/4 };\n```"
);
static EVENTS: KeywordDoc = doc!(
    "events",
    "a quoted event-track composition expression",
    "A `events` quote writes a composition term directly, with `${...}` splicing typed `EventTrack<WrittenTime>` into it. What the quote guarantees is exact extent, closure, and payload typing; what it does not guarantee is that a surface claim made inside a hole still holds after the quote's own `shift`, `scale`, or `restrict` moved it.\n\n\
     ```musa\nlet doubled: EventTrack<WrittenTime> = events EventTrack[WrittenTime, ScoreFact] {\n    let s = ${subject} in together { s; shift by 1/2 s; }\n};\n```"
);
static QUOTE: KeywordDoc = doc!(
    "quote",
    "a syntax quotation, written where an adapter builds",
    "`quote at here { ... }` builds syntax by writing it. The body is read by the ordinary parser, `$x` and \
     `${...}` splice one value where one node stands, `$..xs` splices a list where a sequence stands, and the \
     identity of every node written literally is computed rather than allocated.\n\n\
     ```musa\nquote at here { Sounded(${ anchored(region, here) }, $event, $items) }\n```"
);
static OPTION: KeywordDoc = doc!(
    "Option",
    "a type that may contain one value",
    "`Option<T>` represents an honest partial musical result: either `Some(value)` or `None`, with both cases handled explicitly.\n\n\
     ```musa\nlet found: Option<Pitch> = None;\n```"
);
static LIST: KeywordDoc = doc!(
    "List",
    "a finite ordered collection type",
    "`List<T>` is a finite ordered collection used by total folds and music-theory libraries. Square brackets construct its values.\n\n\
     ```musa\nlet tones: List<Pitch> = [c4, e4, g4];\n```"
);
static RESULT: KeywordDoc = doc!(
    "Result",
    "a value, or the reason there is none",
    "`Result<T, E>` is the binary sum, in the one shape this language has a use for: either `Ok(value)` or `Err(reason)`. \
     Unlike `Option<T>` it says *which* way an operation failed, so an operation with two distinct failures returns one \
     rather than asking the caller to re-derive the reason.\n\n\
     ```musa\nlet series: Result<Row12, (List<Nat>, List<Pc12>)> = row12_of(sketch);\n```"
);
static OK: KeywordDoc = doc!(
    "Ok",
    "a result carrying the value that was wanted",
    "`Ok(value)` constructs the left injection of `Result<T, E>`. It carries its type's capital because it is one of that \
     type's two constructors.\n\n\
     ```musa\nlet found: Result<Pitch, Text> = Ok(c4);\n```"
);
static ERR: KeywordDoc = doc!(
    "Err",
    "a result carrying the reason there is no value",
    "`Err(reason)` constructs the right injection of `Result<T, E>`. The reason is an ordinary value of the error type, not \
     a second channel beside the returned one, so a `match` reads it the way it reads any other case.\n\n\
     ```musa\nlet found: Result<Pitch, Text> = Err(\"no pitch spells that class here\");\n```"
);
static MATCH: KeywordDoc = doc!(
    "match",
    "handle every form of a finite value",
    "A `match` expression names each possible case of an option, list, product, boolean, or other finite value. The checker requires complete, non-overlapping arms.\n\n\
     ```musa\nfn keep(x: Option<Pitch>) -> Option<Pitch> { match x { None -> None, Some(p) -> Some(p), } }\n```"
);
static IF: KeywordDoc = doc!(
    "if",
    "choose between two values by a condition",
    "`if condition { consequent } else { alternative }` is one expression, not a statement: it has a value, both branches have the same type, and the `else` is required. It is written out as the two-arm boolean `match` it stands for, so it costs exactly what that match costs.\n\n\
     ```musa\nfn clef_named(word: Text) -> Clef { if text_equal(word, \"treble\") { Treble } else { Bass } }\n```"
);
static ELSE: KeywordDoc = doc!(
    "else",
    "the other branch of an `if`",
    "`else` introduces the value an `if` takes when its condition is false. It is mandatory — a one-armed conditional would need a value for the case it does not cover, and this language has none. Writing another `if` after it makes a ladder.\n\n\
     ```musa\nif quarters(n) { Quarter } else if halves(n) { Half } else { Whole }\n```"
);
static SOME: KeywordDoc = doc!(
    "Some",
    "an option containing a value",
    "`Some(value)` constructs the present case of an `Option`; a `match` can bind the contained value. It carries its type's capital because it is one of that type's two constructors.\n\n\
     ```musa\nlet tonic: Option<Pitch> = Some(c4);\n```"
);
static NONE: KeywordDoc = doc!(
    "None",
    "an option containing no value",
    "`None` is the absent case of an `Option`. It makes partial musical operations explicit instead of hiding failure. `meter none;` is a different word: a meter that says there are no barlines.\n\n\
     ```musa\nlet absent: Option<Pitch> = None;\n```"
);
static TRUE: KeywordDoc = doc!(
    "true",
    "the affirmative boolean value",
    "`true` is one of the two `Bool` values and can be handled by `match`.\n\n\
     ```musa\nlet enabled: Bool = true;\n```"
);
static FALSE: KeywordDoc = doc!(
    "false",
    "the negative boolean value",
    "`false` is one of the two `Bool` values and can be handled by `match`.\n\n\
     ```musa\nlet muted: Bool = false;\n```"
);

static SCALE: KeywordDoc = doc!(
    "scale",
    "a collection rooted on a note",
    "A scale is an ordered collection of notes rooted on a spelled tonic — `scale c dorian` — and it is a set of \
     pitch coordinates, not a key signature and not a register. `in scale s { … }` reads the music inside it \
     against `s` without writing a key change.\n\n\
     ```musa\nlet home: Scale = scale c major;\nin scale c dorian { use subject; }\n```"
);
static DEGREE: KeywordDoc = doc!(
    "degree",
    "a scale ordinal, with no scale in it",
    "A degree is a numbered position — 1 is the tonic, 8 the tonic a period higher — plus any chromatic \
     alteration, and it belongs to no particular scale until one is supplied. Turning a degree into a note also \
     needs a register, which is what a `Frame` carries. The word is reserved only so that the old spelling of the \
     type gets one complaint carrying `Degree`.\n\n\
     ```musa\nlet dominant: Degree = scale_degree(5);\n```"
);
static FRAME: KeywordDoc = doc!(
    "frame",
    "a scale that knows which octave",
    "A frame is a scale plus the written pitch its first degree sounds, and it is the only thing that can turn a \
     degree into a note. Building one fails unless the tonic pitch spells the scale's own tonic. The word is \
     reserved only so that the old spelling of the type gets one complaint carrying `Frame`.\n\n\
     ```musa\nfn dominant_of(home: Frame) -> Pitch { frame_pitch(home, scale_degree(5)) }\n```"
);
static IN: KeywordDoc = doc!(
    "in",
    "read this music in a scale",
    "`in scale s { … }` supplies the pitch coordinates the enclosed music is read against, for `step` and for \
     anything else that counts scale degrees. It is lexical and local: it emits no key signature and claims no \
     modulation, so the same saved phrase can be used under two scales and mean two things.\n\n\
     ```musa\nin scale c dorian { use subject; }\n```"
);
static STEP: KeywordDoc = doc!(
    "step",
    "move by scale steps, not by an interval",
    "`p step n` moves `n` degrees up the scale in force, and `p step down n` moves down; the answer depends on \
     where in the scale `p` sits, which is exactly how `up M2` differs from it. The note stepped from has to be \
     in the scale, and there has to be a scale.\n\n\
     ```musa\nin scale c major { (c5 step 2)/4 }\n```"
);

static CHORD: KeywordDoc = doc!(
    "chord",
    "rooted spelled content, with no register",
    "`chord c major7` names a root and what is stacked on it, and nothing else: no octave, no spacing, no doubling, \
     and no bass unless one is designated. It does not sound. Choosing the notes a player holds is a voicing policy, \
     which can decline.\n\n\
     ```musa\nlet harmony: chord_class = chord c major7;\n```"
);
static STACK: KeywordDoc = doc!(
    "stack",
    "sound a chord in close position",
    "`stack c4 major7/2` is sugar for the close-position voicing of that chord rooted at that written pitch, held \
     for that long. The pitch is what fixes the register, so `stack c major7/2` is refused: a pitch class chooses \
     no octave.\n\n\
     ```musa\nstack c4 major7/2\n```"
);
static TEMPLATE: KeywordDoc = doc!(
    "template",
    "parameterize a piece or a voice",
    "`template piece study(k: Key) \"Study\" { ... }` writes a family of pieces rather than a piece. Parameters are \
     ordinary typed values — a `Key`, a `Scale`, a `EventTrack<WrittenTime>`, or a `EventTrack<WrittenTime> -> EventTrack<WrittenTime>` — and a template body reads them and \
     the file's root, never the site that makes it. A template is not a value: nothing can pass one, return one, or \
     ask what is inside it.\n\n\
     ```musa\ntemplate voice answer(subject: EventTrack<WrittenTime>, transform: EventTrack<WrittenTime> -> EventTrack<WrittenTime>) {\n    use transform(subject);\n}\n```"
);
static MAKE: KeywordDoc = doc!(
    "make",
    "instantiate a template here",
    "`make study(key g major) as study_in_g;` evaluates the arguments in the scope it is written in and expands the \
     template into an ordinary declaration at this place. A piece instance stands at the file's root and is that \
     file's piece; a voice instance stands among a part's voices. The `as` name is the address; identity comes from \
     the site, so two instances with equal arguments remain two declarations.\n\n\
     ```musa\nmake answer(subject, transpose(P8)) as follower;\n```"
);
static SIGNATURE: KeywordDoc = doc!(
    "signature",
    "name what a module must provide",
    "`signature TonalContext { let key: key; let scale: scale; }` fixes the members a module has to define, and \
     their types. Matching is by name and exact type: a module that satisfies it may define more, and everything \
     unlisted is private to that module. A signature is not a value — nothing can pass one or ask what is inside \
     it.\n\n\
     ```musa\nsignature TonalContext {\n    let key: Key;\n    let scale: Scale;\n}\n```"
);
static MOD: KeywordDoc = doc!(
    "mod",
    "declare one child of a package's module tree",
    "`mod tonal;` says this package has a module called `tonal` — a `tonal.musa` beside this file, or a \
     `tonal/mod.musa` declaring children of its own. A package's tree is its `mod` declarations and nothing else: \
     a `.musa` file no `mod` reaches is not part of the package, and saying so is an error rather than a silence.\n\n\
     ```musa\nmod core;\nmod tonal;\n```"
);
static STRUCTURE: KeywordDoc = doc!(
    "structure",
    "group declarations behind a signature",
    "`structure CMajor : TonalContext { let key = key c major; ... }` names a group of `let` and `fn` declarations, \
     reached from outside as `CMajor.key`. `template structure` parameterizes one over other structures, and `make` \
     applies it. Structures are static: they hold no state, cross no boundary as values, and disappear into ordinary \
     declarations once made.\n\n\
     ```musa\nstructure CMajor : TonalContext {\n    let key = key c major;\n    let scale = scale c major;\n}\n```"
);
static MODULE: KeywordDoc = doc!(
    "module",
    "the old spelling of `structure`",
    "`module` no longer declares anything. What it used to declare is a `structure` — the thing that provides a \
     `signature` — and `module` now means only a node of a package's tree, which is declared with `mod`. The two \
     were one letter apart and unrelated, so the rarer one took the name ML has used for it since 1984.\n\n\
     ```musa\nstructure CMajor : TonalContext {\n    let key = key c major;\n}\n```"
);
static AS: KeywordDoc = doc!(
    "as",
    "name what an instance makes",
    "The mandatory second half of `make`. It is the source address other declarations refer to, and renaming it \
     changes what can be written, not what the instance is.\n\n\
     ```musa\nmake study(key g major) as study_in_g;\n```"
);

static DATA: KeywordDoc = doc!(
    "data",
    "declare a finite type of your own",
    "A `data` declaration is a finite, strictly positive type with named constructors and named fields. It is how a \
     package owns its own data instead of asking the compiler for another built-in type: the constructors are the \
     only way in, `match` is how a reader takes one apart, and the generated fold — `motive_fold` for `data Motive` \
     — is how a total traversal is written.\n\n\
     A field may not be a function, at any depth, and the declared type may not appear to the left of an arrow in \
     its own group. Those two rules are what make the type finite and its fold terminating.\n\n\
     ```musa\ndata Motive {\n    Silence,\n    Sounded(pitch: Pitch, held: Duration),\n}\n```"
);

static RECORD: KeywordDoc = doc!(
    "record",
    "declare a type of named fields",
    "A `record` declaration names fields and their types. It is constructed by naming every field, read by \
     projecting one, matched by naming the ones an arm cares about, and rebuilt by `with` — including along a path, \
     so changing a field of a field is one line rather than a rebuilt inner value.\n\n\
     A record *is* its fields: two declarations with the same field names at the same types are one type, and one is \
     accepted where the other is expected. Where two quantities have to stay apart, declare them as one-case `enum`s \
     instead, because each `enum` generates its own type.\n\n\
     ```musa\nrecord Pending {\n    read: Reading;\n    dots: Dots;\n}\n\nfn refuse(p: Pending, why: Text) -> \
     Pending { p with { read.refusal = why } }\n```"
);

static ENUM: KeywordDoc = doc!(
    "enum",
    "declare a type of alternatives",
    "An `enum` declares alternatives, one case at a time. A case may carry nothing, a positional list of types, or \
     named fields, and `match` over the declared cases is how a reader takes one apart with coverage checked.\n\n\
     Cases live in the type's namespace — `Tying::Untied` — so two enums may declare a case of the same name without \
     colliding. The bare spelling is accepted wherever the expected type is already known, which is where the type \
     says which namespace the word is read in.\n\n\
     ```musa\nenum Tying { Untied, TiedOn }\n\nenum Reading<A> {\n    Done(A),\n    Refused { at: NodePath, why: \
     Text },\n}\n```"
);

static PRIVATE: KeywordDoc = doc!(
    "private",
    "keep a declaration inside its own module",
    "`private` before a declaration makes it nameable only inside the module that writes it; siblings in that module \
     read it by its bare name with no ceremony. Everything is public without the marker, because a musa package is a \
     vocabulary — `std::notation::staff` exists to be named — so hiding is the thing worth writing down.\n\n\
     Before an enum's cases it hides the constructors and leaves the *type* public, which is how a package maintains \
     an invariant: a chord whose symbol has to agree with its tones is built through the function that keeps them in \
     step, and there is no raw constructor to route around it. All the cases or none of them — a mixed enum has no \
     coverage rule worth explaining — and outside the module such a type is not taken apart by `match`, but received \
     from and passed to whatever its package exports.\n\n\
     ```musa\nprivate fn dotted_factor(dots: Dots) -> Ratio { … }\n\nenum Chord {\n    private \
     NamedChord(ChordSymbol, List<Spelling>),\n}\n```"
);
static IMPL: KeywordDoc = doc!(
    "impl",
    "open a type's namespace",
    "`impl T { … }` opens `T`'s namespace: each function in it is the definition `T.f`, reached as `T::f(x)`, as \
     `x.f(…)` where `x`'s type is a `T`, and through whichever operator spells it. The block is a prefix and \
     nothing more — there is no dispatch, and a function written here is a function written at the top level with \
     one more word in its name.\n\n\
     ```musa\nimpl Pitch {\n    fn act(subject: Pitch, operation: Interval) -> Pitch { … }\n}\n```"
);
/// The keyword's documentation, or `None` for anything that is not a
/// keyword.
///
/// Exhaustive by construction, like [`crate::highlight::TokenClass::of`]: the
/// workspace forbids wildcard match arms, so a new [`SyntaxKind`] arrives
/// here as a compile error — and whether it is a keyword is a decision made
/// on purpose, not a default fallen into.
pub fn keyword_doc(kind: SyntaxKind) -> Option<&'static KeywordDoc> {
    let doc = match kind {
        SyntaxKind::ScaleKw => &SCALE,
        SyntaxKind::DegreeKw => &DEGREE,
        SyntaxKind::FrameKw => &FRAME,
        SyntaxKind::InKw => &IN,
        SyntaxKind::StepKw => &STEP,
        SyntaxKind::ChordKw => &CHORD,
        SyntaxKind::StackKw => &STACK,
        SyntaxKind::TemplateKw => &TEMPLATE,
        SyntaxKind::SignatureKw => &SIGNATURE,
        SyntaxKind::StructureKw => &STRUCTURE,
        SyntaxKind::DataKw => &DATA,
        SyntaxKind::RecordKw => &RECORD,
        SyntaxKind::EnumKw => &ENUM,
        SyntaxKind::ModuleKw => &MODULE,
        SyntaxKind::ModKw => &MOD,
        SyntaxKind::PrivateKw => &PRIVATE,
        SyntaxKind::ImplKw => &IMPL,
        SyntaxKind::MakeKw => &MAKE,
        SyntaxKind::AsKw => &AS,
        SyntaxKind::PieceKw => &PIECE,
        SyntaxKind::LibraryKw => &LIBRARY,
        SyntaxKind::TempoKw => &TEMPO,
        SyntaxKind::MeterKw => &METER,
        SyntaxKind::KeyKw => &KEY,
        SyntaxKind::SubtitleKw => &SUBTITLE,
        SyntaxKind::ComposerKw => &COMPOSER,
        SyntaxKind::ArrangerKw => &ARRANGER,
        SyntaxKind::CopyrightKw => &COPYRIGHT,
        SyntaxKind::MotifKw => &MOTIF,
        SyntaxKind::FragmentKw => &FRAGMENT,
        SyntaxKind::ScoreKw => &SCORE,
        SyntaxKind::PartKw => &PART,
        SyntaxKind::VoiceKw => &VOICE,
        SyntaxKind::ClefKw => &CLEF,
        SyntaxKind::ProfileKw => &PROFILE,
        SyntaxKind::PerformanceKw => &PERFORMANCE,
        SyntaxKind::UseKw => &USE,
        SyntaxKind::ImportKw => &IMPORT,
        SyntaxKind::SyntaxKw => &SYNTAX,
        SyntaxKind::TransposeKw => &TRANSPOSE,
        SyntaxKind::UpKw => &UP,
        SyntaxKind::DownKw => &DOWN,
        SyntaxKind::RestKw => &REST,
        SyntaxKind::RepeatKw => &REPEAT,
        SyntaxKind::BarKw => &BAR,
        SyntaxKind::AssertKw => &ASSERT,
        SyntaxKind::EndingKw => &ENDING,
        SyntaxKind::SlurKw => &SLUR,
        SyntaxKind::PhraseKw => &PHRASE,
        SyntaxKind::MarkKw => &MARK,
        SyntaxKind::DynamicKw => &DYNAMIC,
        SyntaxKind::GrooveKw => &GROOVE,
        SyntaxKind::GraceKw => &GRACE,
        SyntaxKind::TupletKw => &TUPLET,
        SyntaxKind::StretchKw => &STRETCH,
        SyntaxKind::RetrogradeKw => &RETROGRADE,
        SyntaxKind::InvertKw => &INVERT,
        SyntaxKind::AroundKw => &AROUND,
        SyntaxKind::WithKw => &WITH,
        SyntaxKind::NoteKw => &NOTE,
        SyntaxKind::SectionKw => &SECTION,
        SyntaxKind::HarmonyKw => &HARMONY,
        SyntaxKind::CrescendoKw => &CRESCENDO,
        SyntaxKind::DiminuendoKw => &DIMINUENDO,
        SyntaxKind::ToKw => &TO,
        SyntaxKind::OverKw => &OVER,
        SyntaxKind::SenzaKw => &SENZA,
        SyntaxKind::MobileKw => &MOBILE,
        SyntaxKind::ImproviseKw => &IMPROVISE,
        SyntaxKind::StudioKw => &STUDIO,
        SyntaxKind::PatchKw => &PATCH,
        SyntaxKind::BusKw => &BUS,
        SyntaxKind::AssignKw => &ASSIGN,
        SyntaxKind::RouteKw => &ROUTE,
        SyntaxKind::SendKw => &SEND,
        SyntaxKind::ModulateKw => &MODULATE,
        SyntaxKind::MasterKw => &MASTER,
        SyntaxKind::AtKw => &AT,
        SyntaxKind::OutputKw => &OUTPUT,
        SyntaxKind::PitchKw => &PITCH_KW,
        SyntaxKind::LetKw => &LET,
        SyntaxKind::FnKw => &FN,
        SyntaxKind::MusicKw => &MUSIC,
        SyntaxKind::EventsKw => &EVENTS,
        SyntaxKind::QuoteKw => &QUOTE,
        SyntaxKind::OptionKw => &OPTION,
        SyntaxKind::ListKw => &LIST,
        SyntaxKind::ResultKw => &RESULT,
        SyntaxKind::MatchKw => &MATCH,
        SyntaxKind::IfKw => &IF,
        SyntaxKind::ElseKw => &ELSE,
        SyntaxKind::SomeKw => &SOME,
        SyntaxKind::NoneKw => &NONE,
        SyntaxKind::OkKw => &OK,
        SyntaxKind::ErrKw => &ERR,
        SyntaxKind::TrueKw => &TRUE,
        SyntaxKind::FalseKw => &FALSE,

        // Everything else — trivia, literals, units, punctuation, and every
        // node kind — is not a keyword and has no doc here.
        SyntaxKind::Whitespace
        | SyntaxKind::LineComment
        | SyntaxKind::BlockComment
        | SyntaxKind::Identifier
        | SyntaxKind::Integer
        | SyntaxKind::Float
        | SyntaxKind::Rational
        | SyntaxKind::String
        | SyntaxKind::PitchLiteral
        | SyntaxKind::IntervalLiteral
        | SyntaxKind::UnitHz
        | SyntaxKind::UnitMs
        | SyntaxKind::UnitS
        | SyntaxKind::UnitDb
        | SyntaxKind::UnitBpm
        | SyntaxKind::LBrace
        | SyntaxKind::RBrace
        | SyntaxKind::LBracket
        | SyntaxKind::RBracket
        | SyntaxKind::LParen
        | SyntaxKind::RParen
        | SyntaxKind::Semicolon
        | SyntaxKind::Comma
        | SyntaxKind::Colon
        | SyntaxKind::Arrow
        | SyntaxKind::PipeForward
        | SyntaxKind::Equals
        | SyntaxKind::Minus
        | SyntaxKind::Tilde
        | SyntaxKind::Dot
        | SyntaxKind::Slash
        | SyntaxKind::Pipe
        | SyntaxKind::Greater
        | SyntaxKind::Less
        | SyntaxKind::Caret
        | SyntaxKind::Hash
        | SyntaxKind::Dollar
        | SyntaxKind::Question
        | SyntaxKind::Error
        | SyntaxKind::Root
        | SyntaxKind::PieceDecl
        | SyntaxKind::TempoStmt
        | SyntaxKind::MeterStmt
        | SyntaxKind::KeyStmt
        | SyntaxKind::FrontMatterStmt
        | SyntaxKind::MotifDecl
        | SyntaxKind::ScoreDecl
        | SyntaxKind::PartDecl
        | SyntaxKind::ClefStmt
        | SyntaxKind::VoiceDecl
        | SyntaxKind::NoteStmt
        | SyntaxKind::RestStmt
        | SyntaxKind::ChordStmt
        | SyntaxKind::UseStmt
        | SyntaxKind::TransposeStmt
        | SyntaxKind::RepeatStmt
        | SyntaxKind::SlurStmt
        | SyntaxKind::DynamicStmt
        | SyntaxKind::TupletStmt
        | SyntaxKind::StretchStmt
        | SyntaxKind::RetrogradeStmt
        | SyntaxKind::InvertStmt
        | SyntaxKind::WithClause
        | SyntaxKind::OverrideStmt
        | SyntaxKind::PhraseStmt
        | SyntaxKind::MarkStmt
        | SyntaxKind::GraceStmt
        | SyntaxKind::GraceNote
        | SyntaxKind::SectionStmt
        | SyntaxKind::HarmonyDecl
        | SyntaxKind::HarmonyStmt
        | SyntaxKind::Position
        | SyntaxKind::PitchClass
        | SyntaxKind::ChordSymbol
        | SyntaxKind::LibraryDecl
        | SyntaxKind::ImportStmt
        | SyntaxKind::SyntaxRegion
        | SyntaxKind::SyntaxGroup
        | SyntaxKind::HairpinStmt
        | SyntaxKind::Duration
        | SyntaxKind::ArticulationList
        | SyntaxKind::PerformanceDecl
        | SyntaxKind::ProfileDecl
        | SyntaxKind::MarkRule
        | SyntaxKind::DynamicRule
        | SyntaxKind::GrooveRule
        | SyntaxKind::GraceRule
        | SyntaxKind::SettingStmt
        | SyntaxKind::ProfileStmt
        | SyntaxKind::StudioDecl
        | SyntaxKind::PatchDecl
        | SyntaxKind::BusDecl
        | SyntaxKind::SignalBinding
        | SyntaxKind::ChainStmt
        | SyntaxKind::SignalChain
        | SyntaxKind::CallExpr
        | SyntaxKind::ArgList
        | SyntaxKind::Arg
        | SyntaxKind::ValueLiteral
        | SyntaxKind::NameRef
        | SyntaxKind::ModulateStmt
        | SyntaxKind::ParamPath
        | SyntaxKind::AssignStmt
        | SyntaxKind::RouteStmt
        | SyntaxKind::SendStmt
        | SyntaxKind::Block
        | SyntaxKind::BarStmt
        | SyntaxKind::AssertStmt
        | SyntaxKind::SenzaStmt
        | SyntaxKind::EndingStmt
        | SyntaxKind::FragmentDecl
        | SyntaxKind::MobileStmt
        | SyntaxKind::ImproviseStmt
        | SyntaxKind::LetDecl
        | SyntaxKind::FnDecl
        | SyntaxKind::Param
        | SyntaxKind::ParamList
        | SyntaxKind::TypeExpr
        | SyntaxKind::TypeName
        | SyntaxKind::FunctionType
        | SyntaxKind::ProductType
        | SyntaxKind::OptionType
        | SyntaxKind::ListType
        | SyntaxKind::ResultType
        | SyntaxKind::NameExpr
        | SyntaxKind::LiteralExpr
        | SyntaxKind::ParenExpr
        | SyntaxKind::BlockExpr
        | SyntaxKind::ProductExpr
        | SyntaxKind::ListExpr
        | SyntaxKind::OptionExpr
        | SyntaxKind::ResultExpr
        | SyntaxKind::ApplyExpr
        | SyntaxKind::LambdaExpr
        | SyntaxKind::PitchExpr
        | SyntaxKind::ExprArgList
        | SyntaxKind::ExprArg
        | SyntaxKind::MatchExpr
        | SyntaxKind::MatchArm
        | SyntaxKind::IfExpr
        | SyntaxKind::RecordUpdateExpr
        | SyntaxKind::FieldUpdate
        | SyntaxKind::FieldPath
        | SyntaxKind::RecordLiteralExpr
        | SyntaxKind::FieldInit
        | SyntaxKind::PathExpr
        | SyntaxKind::RecordPattern
        | SyntaxKind::FieldPattern
        | SyntaxKind::RecordDecl
        | SyntaxKind::FieldDecl
        | SyntaxKind::EnumDecl
        | SyntaxKind::EnumCase
        | SyntaxKind::QuestionExpr
        | SyntaxKind::Pattern
        | SyntaxKind::MusicExpr
        | SyntaxKind::EventsQuote
        | SyntaxKind::EventsHole
        | SyntaxKind::QuoteExpr
        | SyntaxKind::QuotePattern
        | SyntaxKind::Splice
        | SyntaxKind::SequenceSplice
        | SyntaxKind::ScaleExpr
        | SyntaxKind::KeyExpr
        | SyntaxKind::StepExpr
        | SyntaxKind::InScaleStmt
        | SyntaxKind::ChordExpr
        | SyntaxKind::StackStmt
        | SyntaxKind::TemplateDecl
        | SyntaxKind::MakeStmt
        | SyntaxKind::SignatureDecl
        | SyntaxKind::SignatureMember
        | SyntaxKind::StructureDecl
        | SyntaxKind::ModDecl
        | SyntaxKind::DataDecl
        | SyntaxKind::TypeParams
        | SyntaxKind::TypeParam
        | SyntaxKind::DataVariant
        | SyntaxKind::DataField
        | SyntaxKind::AppliedType
        | SyntaxKind::IndexedType
        | SyntaxKind::DataMember
        | SyntaxKind::EqualsEquals
        | SyntaxKind::Plus
        | SyntaxKind::Star
        | SyntaxKind::ImplDecl
        | SyntaxKind::BinaryExpr
        | SyntaxKind::MethodCallExpr
        | SyntaxKind::IndexExpr => return None,
    };
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::keyword_doc;
    use crate::{SyntaxKind, lex};

    /// Every keyword the lexer knows, discovered the way the lexer reports
    /// it: the table's spelling, fed back through `lex`, must come back as
    /// the kind the table filed it under. A spelling that drifts from the
    /// logos table fails here, not in an editor.
    #[test]
    fn every_doc_spelling_round_trips_through_the_lexer() {
        let kinds = [
            SyntaxKind::PieceKw,
            SyntaxKind::LibraryKw,
            SyntaxKind::TempoKw,
            SyntaxKind::MeterKw,
            SyntaxKind::KeyKw,
            SyntaxKind::SubtitleKw,
            SyntaxKind::ComposerKw,
            SyntaxKind::ArrangerKw,
            SyntaxKind::CopyrightKw,
            SyntaxKind::MotifKw,
            SyntaxKind::FragmentKw,
            SyntaxKind::ScoreKw,
            SyntaxKind::PartKw,
            SyntaxKind::VoiceKw,
            SyntaxKind::ClefKw,
            SyntaxKind::ProfileKw,
            SyntaxKind::PerformanceKw,
            SyntaxKind::UseKw,
            SyntaxKind::ImportKw,
            SyntaxKind::ModKw,
            SyntaxKind::StructureKw,
            SyntaxKind::DataKw,
            SyntaxKind::RecordKw,
            SyntaxKind::EnumKw,
            SyntaxKind::ModuleKw,
            SyntaxKind::PrivateKw,
            SyntaxKind::ImplKw,
            SyntaxKind::TransposeKw,
            SyntaxKind::UpKw,
            SyntaxKind::DownKw,
            SyntaxKind::RestKw,
            SyntaxKind::RepeatKw,
            SyntaxKind::BarKw,
            SyntaxKind::AssertKw,
            SyntaxKind::EndingKw,
            SyntaxKind::SlurKw,
            SyntaxKind::PhraseKw,
            SyntaxKind::MarkKw,
            SyntaxKind::DynamicKw,
            SyntaxKind::GrooveKw,
            SyntaxKind::GraceKw,
            SyntaxKind::TupletKw,
            SyntaxKind::StretchKw,
            SyntaxKind::RetrogradeKw,
            SyntaxKind::InvertKw,
            SyntaxKind::AroundKw,
            SyntaxKind::WithKw,
            SyntaxKind::NoteKw,
            SyntaxKind::SectionKw,
            SyntaxKind::HarmonyKw,
            SyntaxKind::CrescendoKw,
            SyntaxKind::DiminuendoKw,
            SyntaxKind::ToKw,
            SyntaxKind::OverKw,
            SyntaxKind::SenzaKw,
            SyntaxKind::MobileKw,
            SyntaxKind::ImproviseKw,
            SyntaxKind::StudioKw,
            SyntaxKind::PatchKw,
            SyntaxKind::BusKw,
            SyntaxKind::AssignKw,
            SyntaxKind::RouteKw,
            SyntaxKind::SendKw,
            SyntaxKind::ModulateKw,
            SyntaxKind::MasterKw,
            SyntaxKind::AtKw,
            SyntaxKind::OutputKw,
            SyntaxKind::PitchKw,
            SyntaxKind::LetKw,
            SyntaxKind::FnKw,
            SyntaxKind::MusicKw,
            SyntaxKind::QuoteKw,
            SyntaxKind::OptionKw,
            SyntaxKind::ListKw,
            SyntaxKind::MatchKw,
            SyntaxKind::IfKw,
            SyntaxKind::ElseKw,
            SyntaxKind::SomeKw,
            SyntaxKind::NoneKw,
            SyntaxKind::TrueKw,
            SyntaxKind::FalseKw,
            SyntaxKind::ScaleKw,
            SyntaxKind::DegreeKw,
            SyntaxKind::FrameKw,
            SyntaxKind::InKw,
            SyntaxKind::StepKw,
            SyntaxKind::ChordKw,
            SyntaxKind::StackKw,
        ];
        for kind in kinds {
            let doc = keyword_doc(kind);
            assert!(doc.is_some(), "{kind:?} has no doc");
            let Some(doc) = doc else { continue };
            assert!(!doc.summary.is_empty(), "{kind:?}: empty summary");
            assert!(doc.doc.contains("```musa"), "{kind:?}: doc has no example");
            // One assertion for both halves of the law: the spelling lexes as
            // exactly one token, and that token is the kind it was filed
            // under.
            let lexed: Vec<SyntaxKind> = lex(doc.spelling).tokens().iter().map(|token| token.kind).collect();
            assert_eq!(lexed, [kind], "{kind:?}: `{}` drifted from the lexer", doc.spelling);
        }
    }
}
