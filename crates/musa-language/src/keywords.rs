//! Every keyword's own plain-English documentation (prompt 84).
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
     rather than played itself. A piece brings one in with `use \"path\";` and then speaks its names as its own.\n\n\
     ```musa\nlibrary { motif sigh(root: pitch) { … } }\n```"
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
     ```musa\nmotif sigh(root: pitch) { root 1/8; root 1/8; }\n```"
);
static FRAGMENT: KeywordDoc = doc!(
    "fragment",
    "a labeled block a mobile can arrange",
    "A fragment is a named block of music that means nothing in source order: it becomes music when a `mobile` \
     arranges it among others. Fragments are how musa writes music whose order is chosen, not fixed.\n\n\
     ```musa\nfragment a { c4 1/4; d4 1/4; }\n```"
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
     ```musa\nvoice right { c4 1/4; d4 1/4; }\n```"
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
    "write out a motif, or import a library",
    "`use` writes out a declared motif where it stands, with arguments for its parameters: `use sigh(e5);` is \
     the motif spelled here, once. At the top of a piece it imports a library file instead: `use \"strings\";`.\n\n\
     ```musa\nuse sigh(e5);\n```"
);
static TRANSPOSE: KeywordDoc = doc!(
    "transpose",
    "the same music, moved in pitch",
    "A transpose block plays its contents moved by an interval — `up` or `down` a named interval like `P5` or \
     `M3`. The original is untouched; the block is a reading of it at a new pitch.\n\n\
     ```musa\ntranspose up P5 { use theme(); }\n```"
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
     ```musa\nrest 1/4;\n```"
);
static CHORD: KeywordDoc = doc!(
    "chord",
    "several pitches as one rhythmic thing",
    "A chord is several pitches struck as one rhythmic event: `chord [c4 e4 g4] 1/4;` fills a quarter with all \
     of them. It is one voice's one event, not three voices.\n\n\
     ```musa\nchord [c4 e4 g4] 1/2;\n```"
);
static REPEAT: KeywordDoc = doc!(
    "repeat",
    "play the block again",
    "A repeat block plays its contents more than once: `repeat 4 { … }` is four passes through the same bars, \
     written once. With `ending` blocks inside, the passes differ where the endings say they do.\n\n\
     ```musa\nrepeat 2 { use verse(); }\n```"
);
static BAR: KeywordDoc = doc!(
    "bar",
    "one measure, written out",
    "A bar is one measure of music between braces: what a player reads between two barlines, and the unit that \
     must add up to the meter. Named — `bar head { … }` — it also becomes an address an edit or a reader can \
     point at.\n\n\
     ```musa\nbar { c4 1/4; d4 1/4; e4 1/4; f4 1/4; }\n```"
);
static ENDING: KeywordDoc = doc!(
    "ending",
    "what changes on a given pass of a repeat",
    "An ending block plays only on the numbered pass of the enclosing `repeat`: `ending 1 { … }` the first \
     time, `ending 2 { … }` the second. It is how a repeated strain gets a different close.\n\n\
     ```musa\nrepeat 2 { use strain(); ending 1 { c4 1/1; } ending 2 { g4 1/1; } }\n```"
);
static SLUR: KeywordDoc = doc!(
    "slur",
    "a phrase mark over the block",
    "A slur block is played legato under one bow or breath: the notation curve over its contents. It changes \
     how the notes connect, not which notes they are.\n\n\
     ```musa\nslur { c4 1/8; d4 1/8; e4 1/8; }\n```"
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
     ```musa\ngrace { d5 1/16; } c5 1/4;\n```"
);
static TUPLET: KeywordDoc = doc!(
    "tuplet",
    "notes in the time of fewer",
    "A tuplet block fits its contents into the time a different count would take: `tuplet 3/2 { … }` is three \
     in the time of two. The ratio is exact — tuplets are arithmetic, not feel.\n\n\
     ```musa\ntuplet 3/2 { c4 1/8; d4 1/8; e4 1/8; }\n```"
);
static STRETCH: KeywordDoc = doc!(
    "stretch",
    "the same music, at a different speed ratio",
    "A stretch block plays its contents scaled in time by an exact factor: `stretch 3/2 { … }` takes half \
     again as long. Rhythm and proportions are preserved; only the clock changes.\n\n\
     ```musa\nstretch 2/1 { use theme(); }\n```"
);
static RETROGRADE: KeywordDoc = doc!(
    "retrograde",
    "the block, backwards",
    "A retrograde block plays its contents in reverse order — the last note first. It is the classical \
     transformation, written as a block rather than spelled out by hand.\n\n\
     ```musa\nretrograde { use subject(); }\n```"
);
static INVERT: KeywordDoc = doc!(
    "invert",
    "the block, mirrored in pitch",
    "An invert block mirrors its contents in pitch around an axis note: what went up goes down by the same \
     interval. The axis is written with `around`.\n\n\
     ```musa\ninvert around c5 { use subject(); }\n```"
);
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
     ```musa\nuse theme() with (pickup: note c5 1/8);\n```"
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
     `harmony { \"Dm7\" 1/1; \"G7\" 1/1; }`. The symbols are parsed, so later tooling reads structure instead \
     of letters.\n\n\
     ```musa\nharmony { \"Cmaj7\" 2/1; }\n```"
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
     ```musa\nsenza { c4 1/1; g4 1/1; }\n```"
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
    "the pitch parameter type",
    "`pitch` is the type of a pitch parameter in a motif declaration: `motif sigh(root: pitch)` takes a written \
     pitch like `e5` at each `use`. It is one of the small set of types parameters can have.\n\n\
     ```musa\nmotif call(root: pitch = c5) { root 1/4; }\n```"
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
        SyntaxKind::TransposeKw => &TRANSPOSE,
        SyntaxKind::UpKw => &UP,
        SyntaxKind::DownKw => &DOWN,
        SyntaxKind::RestKw => &REST,
        SyntaxKind::ChordKw => &CHORD,
        SyntaxKind::RepeatKw => &REPEAT,
        SyntaxKind::BarKw => &BAR,
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
        | SyntaxKind::ChordSymbol
        | SyntaxKind::LibraryDecl
        | SyntaxKind::ImportStmt
        | SyntaxKind::HairpinStmt
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
        | SyntaxKind::SenzaStmt
        | SyntaxKind::EndingStmt
        | SyntaxKind::FragmentDecl
        | SyntaxKind::MobileStmt
        | SyntaxKind::ImproviseStmt => return None,
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
            SyntaxKind::TransposeKw,
            SyntaxKind::UpKw,
            SyntaxKind::DownKw,
            SyntaxKind::RestKw,
            SyntaxKind::ChordKw,
            SyntaxKind::RepeatKw,
            SyntaxKind::BarKw,
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
        ];
        for kind in kinds {
            let doc = keyword_doc(kind).unwrap_or_else(|| panic!("{kind:?} has no doc"));
            assert!(!doc.summary.is_empty(), "{kind:?}: empty summary");
            assert!(doc.doc.contains("```musa"), "{kind:?}: doc has no example");
            let tokens = lex(doc.spelling);
            let [token] = tokens.tokens() else {
                panic!("{:?}: `{}` did not lex as one token", kind, doc.spelling);
            };
            assert_eq!(token.kind, kind, "{kind:?}: spelling drifted from the lexer");
        }
    }
}
