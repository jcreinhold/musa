/**
 * tree-sitter grammar for musa.
 *
 * Every rule here traces to a function in `crates/musa-language/src/parser.rs`
 * — the authoritative, hand-written parser — and every token to a regex or
 * literal in `crates/musa-language/src/lexer.rs`. Node names mirror
 * `syntax_kind.rs`, snake_cased, so the query files read in the language's
 * own vocabulary. Where the trees disagree, the hand parser is right and
 * this file changes: the drift law in
 * `crates/musa-language/tests/tree_sitter_fixtures.rs` is what notices.
 *
 * The grammar needs no external scanner: semicolons and braces are explicit
 * (roadmap §7), so the one-token lookahead LR(1) gives is enough for the
 * three places the hand parser peeks (`name =`, `name(`, bare `name`).
 */

/// <summary>
/// The mirror image of `Parser::voice_items`: one statement per first token,
/// as the hand parser dispatches.
/// </summary>
const VOICE_ITEMS = ($) => [
  $.note_statement,
  $.rest_statement,
  $.chord_statement,
  $.use_statement,
  $.transpose_statement,
  $.repeat_statement,
  $.bar_statement,
  $.grace_statement,
  $.mobile_statement,
  $.improvise_statement,
  // The same statements the header and the part write, written where the
  // music reaches them: one kind, one node, two places (Parser::voice_items).
  $.tempo_statement,
  $.meter_statement,
  $.key_statement,
  $.clef_statement,
  $.ending_statement,
  $.slur_statement,
  $.dynamic_statement,
  $.tuplet_statement,
  $.stretch_statement,
  $.retrograde_statement,
  $.invert_statement,
  $.phrase_statement,
  $.mark_statement,
  $.hairpin_statement,
  $.senza_statement,
];

module.exports = grammar({
  name: 'musa',

  extras: ($) => [/\s/, $.comment],

  // `identifier` as the word token steers error recovery toward spelling
  // mistakes, which is most of what a composer types mid-word.
  word: ($) => $.identifier,

  rules: {
    // Parser::run — a file is a piece or a library, written at the top.
    source_file: ($) => choice($.piece_declaration, $.library_declaration),

    // --- Piece level (Parser::piece_decl) -------------------------------

    piece_declaration: ($) =>
      seq(
        'piece',
        field('name', $.string),
        '{',
        repeat(
          choice(
            $.import_statement,
            $.tempo_statement,
            $.meter_statement,
            $.key_statement,
            $.front_matter_statement,
            $.motif_declaration,
            $.fragment_declaration,
            $.score_declaration,
            $.performance_declaration,
            $.studio_declaration,
          ),
        ),
        '}',
      ),

    // Parser::library_decl — what can be shared: no score, no header.
    library_declaration: ($) =>
      seq(
        'library',
        '{',
        repeat(
          choice(
            $.import_statement,
            $.motif_declaration,
            $.fragment_declaration,
            $.performance_declaration,
            $.studio_declaration,
          ),
        ),
        '}',
      ),

    // Parser::import_stmt — `use "../library/motifs.musa";`
    import_statement: ($) => seq('use', field('path', $.string), ';'),

    // Parser::front_matter_stmt — one shape, four heads.
    front_matter_statement: ($) =>
      seq(choice('subtitle', 'composer', 'arranger', 'copyright'), field('value', $.string), ';'),

    // Parser::tempo_stmt — a metronome mark, a tempo word, or both; a
    // gradual change adds where it arrives and how far it takes to get
    // there (prompt 73).
    tempo_statement: ($) =>
      seq(
        'tempo',
        choice(
          // The word may lead (`tempo "Andante";`) — and a gradual change
          // may still reach from it.
          seq(field('word', $.string), optional($.tempo_span)),
          seq(
            optional(field('beat', choice($.identifier, $.rational))),
            '=',
            field('bpm', $.integer),
            // `to 60` — where a gradual change arrives, in the same beat
            // unit. `to` rather than a keyword of its own, for the reason
            // `crescendo to f` reads: arriving somewhere is one idea.
            optional(seq('to', field('arrives', $.integer))),
            optional($.tempo_span),
            // The word may trail, for a marking that says both.
            optional(field('word', $.string)),
          ),
        ),
        ';',
      ),

    // `over 4/1` — how far a gradual change reaches.
    tempo_span: ($) => seq('over', field('span', $.rational)),

    // Parser::meter_stmt — `meter 4/4;`, or `meter none;` for music with no
    // barlines from here (prompt 74). Any identifier parses; the compiler
    // checks the word, because there is nothing else `meter` can be
    // followed by.
    meter_statement: ($) => seq('meter', field('meter', choice($.rational, $.identifier)), ';'),

    // Parser::key_stmt — `key a minor;`
    key_statement: ($) => seq('key', field('pitch_class', $.identifier), field('mode', $.identifier), ';'),

    // Parser::motif_decl — parameters inline, as in the hand parser: a
    // comma-joined run of `name: type (= default)?`, trailing comma allowed.
    motif_declaration: ($) =>
      seq(
        'motif',
        field('name', $.identifier),
        '(',
        optional(
          seq(
            $.motif_parameter,
            repeat(seq(',', $.motif_parameter)),
            optional(','),
          ),
        ),
        ')',
        field('body', $.block),
      ),

    // One `name: type (= default)?` of Parser::motif_decl's loop.
    motif_parameter: ($) =>
      seq(
        field('name', $.identifier),
        ':',
        field('type', choice('pitch', $.identifier)),
        optional(seq('=', field('default', choice($.pitch_literal, $.rational, $.integer)))),
      ),

    // Parser::fragment_decl — a motif without parameters, tagged differently.
    fragment_declaration: ($) => seq('fragment', field('name', $.identifier), field('body', $.block)),

    // --- Score level (Parser::score_decl and below) ---------------------

    score_declaration: ($) =>
      seq(
        'score',
        '{',
        repeat(choice($.part_declaration, $.section_statement, $.harmony_declaration)),
        '}',
      ),

    // Parser::part_decl — a part carries its own clef, and may carry its
    // own meter and tempo: polymeter and polytempo (prompt 75).
    part_declaration: ($) =>
      seq(
        'part',
        field('name', $.identifier),
        '{',
        repeat(
          choice(
            $.clef_statement,
            $.meter_statement,
            $.tempo_statement,
            $.profile_statement,
            $.voice_declaration,
          ),
        ),
        '}',
      ),

    // Parser::clef_stmt.
    clef_statement: ($) => seq('clef', field('name', $.identifier), ';'),

    // Parser::profile_stmt — which profile realizes this part.
    profile_statement: ($) => seq('profile', field('name', $.identifier), ';'),

    // Parser::voice_decl — the braces are the declaration's own, no Block.
    voice_declaration: ($) =>
      seq('voice', field('name', $.identifier), '{', repeat(choice(...VOICE_ITEMS($))), '}'),

    // --- Performance level (Parser::performance_decl) -------------------

    performance_declaration: ($) => seq('performance', '{', repeat($.profile_declaration), '}'),

    profile_declaration: ($) =>
      seq(
        'profile',
        field('name', $.identifier),
        '{',
        repeat(choice($.mark_rule, $.dynamic_rule, $.groove_rule, $.grace_rule)),
        '}',
      ),

    // Parser::rule — `mark|dynamic|groove <name> { <setting>* }`.
    mark_rule: ($) => seq('mark', field('name', $.identifier), field('body', $.settings_block)),
    dynamic_rule: ($) => seq('dynamic', field('name', $.identifier), field('body', $.settings_block)),
    groove_rule: ($) => seq('groove', field('name', $.identifier), field('body', $.settings_block)),

    // Parser::grace_rule — the same block with no name in front.
    grace_rule: ($) => seq('grace', field('body', $.settings_block)),

    // Parser::settings_block.
    settings_block: ($) => seq('{', repeat($.setting_statement), '}'),

    // Parser::setting_stmt — `<name> = [-]<number or word> [ms|s];`
    setting_statement: ($) =>
      seq(
        field('name', $.identifier),
        '=',
        optional('-'),
        field('value', choice($.float, $.integer, $.rational, $.identifier)),
        optional(field('unit', $.unit)),
        ';',
      ),

    // --- Studio level (Parser::studio_decl and below, roadmap §7.1) -----

    studio_declaration: ($) =>
      seq(
        'studio',
        '{',
        repeat(
          choice(
            $.patch_declaration,
            $.bus_declaration,
            $.modulate_statement,
            $.assign_statement,
            $.route_statement,
            $.send_statement,
            $.signal_binding,
          ),
        ),
        '}',
      ),

    patch_declaration: ($) =>
      seq('patch', field('name', $.identifier), '{', repeat(choice($.signal_binding, $.chain_statement)), '}'),

    bus_declaration: ($) =>
      seq('bus', field('name', $.identifier), '{', repeat(choice($.signal_binding, $.chain_statement)), '}'),

    // Parser::signal_binding — `<name> = <chain>;`
    signal_binding: ($) => seq(field('name', $.identifier), '=', $.signal_chain, ';'),

    // Parser::chain_stmt — unnamed, so its value is the enclosing block's.
    chain_statement: ($) => seq($.signal_chain, ';'),

    // Parser::signal_chain — `|>` is left-associative and the only operator.
    signal_chain: ($) => seq($.stage, repeat(seq('|>', $.stage))),

    // Parser::stage — a construction, a bare name, or the `output` terminal.
    stage: ($) => choice($.call_expression, $.name_reference),

    // Parser::call_expr.
    call_expression: ($) => seq(field('name', $.identifier), $.argument_list),

    argument_list: ($) => seq('(', optional(seq($.argument, repeat(seq(',', $.argument)))), ')'),

    // Parser::arg — `<name>: <value>` or a positional `<value>`.
    argument: ($) => seq(optional(seq(field('name', $.identifier), ':')), choice($.value_literal, $.call_expression, $.name_reference)),

    // Parser::value's number form — `[-]<number> [unit]`.
    value_literal: ($) =>
      seq(optional('-'), choice($.float, $.integer, $.rational), optional(field('unit', $.unit))),

    // Parser's NameRef: another signal, or the `output` / `master` terminal.
    name_reference: ($) => choice('output', 'master', $.identifier),

    // Parser::modulate_stmt — `modulate <signal> -> <patch>.<stage>.<param>;`
    modulate_statement: ($) =>
      seq('modulate', field('signal', $.identifier), '->', $.parameter_path, ';'),

    parameter_path: ($) => seq(field('patch', $.identifier), repeat(seq('.', $.identifier))),

    // Parser::binding_stmt — one shape, two heads.
    assign_statement: ($) =>
      seq('assign', field('source', $.identifier), '->', field('destination', $.identifier), ';'),
    route_statement: ($) =>
      seq('route', field('source', $.identifier), '->', field('destination', choice('master', $.identifier)), ';'),

    // Parser::send_stmt — `send <source> -> <bus> at <gain>;`
    send_statement: ($) =>
      seq(
        'send',
        field('source', $.identifier),
        '->',
        field('bus', $.identifier),
        'at',
        field('gain', choice($.value_literal, $.call_expression, $.name_reference)),
        ';',
      ),

    // --- Voice items (Parser::voice_items and its callees) ---------------

    // Parser::block — the `{ ... }` body of a motif, transpose, or repeat.
    block: ($) => seq('{', repeat(choice(...VOICE_ITEMS($))), '}'),

    // Parser::note_stmt — `<pitch-or-ref> <duration> <articulation>* ~? ;`
    note_statement: ($) =>
      seq(
        field('pitch', choice($.pitch_literal, $.identifier)),
        $.duration,
        optional($.articulation_list),
        optional('~'),
        ';',
      ),

    // Parser::articulations — their own node, by the hand parser's own rule:
    // a bare identifier here is an articulation, not a reference.
    articulation_list: ($) => repeat1($.identifier),

    // Parser::duration — `1/4`, `1`, `/4`, `/4.`, or a parameter reference;
    // `to` bounds how long the written value may be held (roadmap §2).
    // Augmentation dots follow the short form only: `3/8.` is refused because
    // the long form already writes 9/16.
    duration: ($) =>
      seq($._duration_value, optional(seq('to', $._duration_value))),

    _duration_value: ($) =>
      choice(
        seq('/', $.integer, repeat('.')),
        $.rational,
        $.integer,
        $.identifier,
      ),

    rest_statement: ($) => seq('rest', $.duration, ';'),

    // Parser::chord_stmt — `chord [<pitch>, ...] <duration>;`
    chord_statement: ($) =>
      seq(
        'chord',
        '[',
        $.pitch_literal,
        repeat(seq(',', $.pitch_literal)),
        ']',
        $.duration,
        optional($.articulation_list),
        optional('~'),
        ';',
      ),

    // Parser::use_stmt — the parentheses *are* the argument list; `with` is a
    // block, so it takes no `;`.
    use_statement: ($) =>
      seq(
        'use',
        field('name', $.identifier),
        optional(
          seq(
            '(',
            optional(
              seq(
                choice($.pitch_literal, $.identifier, $.rational, $.integer),
                repeat(seq(',', choice($.pitch_literal, $.identifier, $.rational, $.integer))),
              ),
            ),
            ')',
          ),
        ),
        choice(field('overrides', $.with_clause), ';'),
      ),

    // Parser::with_clause — `with { note <n> = <pitch>; ... }`
    with_clause: ($) => seq('with', '{', repeat($.override_statement), '}'),

    override_statement: ($) =>
      seq('note', field('index', $.integer), '=', field('pitch', $.pitch_literal), ';'),

    // Parser::transpose_stmt — `transpose up|down <interval> { ... }`
    transpose_statement: ($) =>
      seq('transpose', field('direction', choice('up', 'down')), field('interval', $.interval_literal), field('body', $.block)),

    // Parser::repeat_stmt — a count, or a range the realization chooses in.
    repeat_statement: ($) =>
      seq('repeat', field('count', $.integer), optional(seq('to', field('maximum', $.integer))), field('body', $.block)),

    // Parser::ending_stmt — `ending 1 { ... }`
    ending_statement: ($) => seq('ending', field('pass', $.integer), field('body', $.block)),

    // Parser::bar_stmt — the name is optional and nothing to disambiguate.
    bar_statement: ($) => seq('bar', optional(field('name', $.identifier)), field('body', $.block)),

    slur_statement: ($) => seq('slur', field('body', $.block)),

    // Parser::mark_stmt — the parser accepts the shape; the compiler checks
    // the vocabulary.
    mark_statement: ($) =>
      seq(
        'mark',
        field('name', $.identifier),
        optional(field('argument', choice($.string, $.integer, seq('-', $.integer)))),
        choice(field('body', $.block), ';'),
      ),

    // Parser::grace_stmt — pitches and nothing else: no written duration.
    grace_statement: ($) => seq('grace', '{', repeat($.grace_note), '}'),

    grace_note: ($) =>
      seq(field('pitch', choice($.pitch_literal, $.identifier)), optional($.articulation_list), ';'),

    // Parser::phrase_stmt — `phrase "A" { ... }`
    phrase_statement: ($) => seq('phrase', field('name', $.string), field('body', $.block)),

    // Parser::hairpin_stmt — `crescendo to f { ... }`
    hairpin_statement: ($) =>
      seq(choice('crescendo', 'diminuendo'), 'to', field('dynamic', $.identifier), field('body', $.block)),

    // Parser::dynamic_stmt — `dynamic <mark>;`
    dynamic_statement: ($) => seq('dynamic', field('mark', $.identifier), ';'),

    // Parser::senza_stmt — `senza { ... }`: the barlines stop for exactly as
    // long as the block, then the meter returns (prompt 74). It is the two
    // meter changes a composer could write by hand, with the second one
    // impossible to forget.
    senza_statement: ($) => seq('senza', field('body', $.block)),

    // Parser::stretch_stmt — `stretch <n>/<d> { ... }`
    stretch_statement: ($) => seq('stretch', field('factor', choice($.rational, $.integer)), field('body', $.block)),

    retrograde_statement: ($) => seq('retrograde', field('body', $.block)),

    // Parser::invert_stmt — `invert around <pitch> { ... }`
    invert_statement: ($) => seq('invert', 'around', field('axis', $.pitch_literal), field('body', $.block)),

    // Parser::tuplet_stmt — `tuplet <n>/<d> { ... }`
    tuplet_statement: ($) => seq('tuplet', field('ratio', $.rational), field('body', $.block)),

    // Parser::mobile_stmt — a list of names, not a block of music.
    mobile_statement: ($) => seq('mobile', '{', repeat(seq(field('fragment', $.identifier), ';')), '}'),

    // Parser::improvise_stmt — `improvise 8/1 over "Dm7 | G7";`
    improvise_statement: ($) =>
      seq('improvise', $.duration, optional(seq('over', field('changes', $.string))), ';'),

    // --- Form markers (Parser::section_stmt, harmony_decl) ---------------

    // Parser::section_stmt — `section "Exposition" at 1:1;`
    section_statement: ($) => seq('section', field('name', $.string), 'at', $.position, ';'),

    harmony_declaration: ($) => seq('harmony', '{', repeat($.harmony_statement), '}'),

    // Parser::harmony_stmt — `at 1:1 am;`
    harmony_statement: ($) => seq('at', $.position, $.chord_symbol, ';'),

    // Parser::position — `<measure>:<beat>`
    position: ($) => seq(field('measure', $.integer), ':', field('beat', choice($.integer, $.rational))),

    // Parser::chord_symbol — one word, however it lexes.
    chord_symbol: ($) => seq(choice($.identifier, $.pitch_literal), optional($.integer)),

    // --- Tokens (crates/musa-language/src/lexer.rs) ----------------------

    // `[a-g](ss|ff|[sfn])?-?[0-9]+` — a written pitch: letter, accidental,
    // octave.
    pitch_literal: ($) => /[a-g](ss|ff|[sfn])?-?[0-9]+/,

    // `[PMm][0-9]+` — an interval: quality and size.
    interval_literal: ($) => /[PMm][0-9]+/,

    rational: ($) => /[0-9]+\/[0-9]+/,
    float: ($) => /[0-9]+\.[0-9]+/,
    integer: ($) => /[0-9]+/,
    identifier: ($) => /[a-zA-Z_]+/,

    // `"([^"\\\n]|\\[^\n])*"` — one quoted line. An unterminated quote is
    // error recovery's business, exactly as the hand lexer leaves it.
    string: ($) => /"([^"\\\n]|\\[^\n])*"/,

    // The units the parser's rules consume: `Hz`, `ms`, `s`, `dB`. (`bpm`
    // lexes but no rule reads it yet; it joins when one does.)
    unit: ($) => token(choice('Hz', 'ms', 's', 'dB')),

    comment: ($) =>
      choice(
        // `//[^\n]*`
        /\/\/[^\n]*/,
        // `(?s)/\*([^*]|\*[^/])*\*/`
        // `(?s)/\*([^*]|\*[^/])*\*/` — the flag needs no spelling here: the
        // classes are negations, which match newlines in both regex flavors.
        /\/\*([^*]|\*[^/])*\*\//,
      ),
  },
});
