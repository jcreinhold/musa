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
/// The statements a drawn bar (`|`) accepts: the subset of VOICE_ITEMS that
/// runs to the next `|`.
/// </summary>
const BAR_ITEMS = ($) => [
  $.note_statement,
  $.rest_statement,
  $.chord_statement,
  $.use_statement,
  $.dynamic_statement,
  $.clef_statement,
  $.tempo_statement,
  $.mark_statement,
  $.hairpin_statement,
  $.tuplet_statement,
  $.slur_statement,
  $.grace_statement,
];

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
  $.in_scale_statement,
  $.assert_statement,
  $.stack_statement,
];

module.exports = grammar({
  name: 'musa',

  extras: ($) => [/\s/, $.comment],

  // An event ends where the next one starts, and telling those apart takes
  // more than one token of lookahead: after `root/8 tenuto`, whether `tenuto`
  // is this note's articulation or the next note's pitch is settled by what
  // follows it. The hand parser looks ahead one *kind*; here the GLR parser
  // explores both and keeps the reading that parses.
  conflicts: ($) => [
    [$.note_statement],
    [$.chord_statement],
    [$.stack_statement],
    [$.grace_note],
    [$.articulation_list],
  ],

  // `identifier` as the word token steers error recovery toward spelling
  // mistakes, which is most of what a composer types mid-word.
  word: ($) => $.identifier,

  rules: {
    // Parser::run — a file is a piece or a library, written at the top,
    // behind whatever its lexical root declares. A `make` of a piece
    // template stands where the piece would: one file is still one piece.
    // A module file is the exception: `src/lib.musa` and a directory's
    // `mod.musa` declare a package's children and nothing else, so they carry
    // no piece and no library (Parser::root_preamble).
    source_file: ($) =>
      choice(
        $.kernel_document,
        seq(
          repeat(
            choice(
              $.import_statement,
              $.let_declaration,
              $.function_declaration,
              $.template_declaration,
              $.signature_declaration,
              $.structure_declaration,
              $.make_statement,
            ),
          ),
          choice($.piece_declaration, $.library_declaration, $.make_statement),
        ),
        repeat1($.mod_declaration),
      ),

    // The kernel alternative (`docs/language/01-surface.md` §7): a file whose
    // first line is the interchange version marker.
    //
    // **Recognized, not parsed.** `musa-kernel` owns the term grammar, and a
    // second copy of it here would be exactly the drifting duplicate this
    // grammar's own README says it must not become — with the added problem
    // that no lexer exists to hold it to, since the kernel's reader is a
    // hand-written cursor rather than a token stream. So the marker is a node
    // an editor can query and the rest is one opaque span. What colours inside
    // it is `musa-kernel`'s own classification, delivered as LSP semantic
    // tokens; what this rule buys is that a `.musa.kernel` file opened in a
    // tree-sitter editor is a document rather than a page of red.
    kernel_document: ($) => seq($.kernel_marker, optional($.kernel_body)),

    kernel_marker: (_) => token(prec(2, seq('%', /[ \t]*/, 'musa-kernel-1', /[ \t]*/, /\r?\n/))),

    kernel_body: (_) => token(prec(-1, /[\s\S]+/)),

    // Parser::mod_decl — one child of the package's module tree. A name and
    // nothing else: what the name reaches is a fact about the package's
    // files, which no parser has.
    mod_declaration: ($) => seq('mod', field('name', $._module_name), ';'),

    // Parser::template_decl — the word that says a declaration is a pattern.
    // The declaration it parameterizes is its only child, so every query
    // written for a piece or a voice still matches inside one.
    template_declaration: ($) =>
      seq('template', choice($.piece_declaration, $.voice_declaration, $.structure_declaration)),

    // Parser::signature_decl — what a structure must provide. A member is a
    // `let` with its definition left out, because a function is a value of
    // arrow type and one member form covers all of them.
    signature_declaration: ($) =>
      seq('signature', field('name', $.identifier), '{', repeat($.signature_member), '}'),

    signature_member: ($) => seq('let', field('name', $.identifier), ':', field('type', $.type_expression), ';'),

    // Parser::structure_decl — a named group of declarations, reached from
    // outside as `M.member`. The parameter list is what a
    // `template structure` adds, and nothing else about the node changes.
    structure_declaration: ($) =>
      seq(
        'structure',
        field('name', $.identifier),
        optional($.parameter_list),
        ':',
        field('signature', $.identifier),
        '{',
        repeat(choice($.let_declaration, $.function_declaration)),
        '}',
      ),

    // Parser::make_stmt — one instance site.
    make_statement: ($) =>
      seq(
        'make',
        field('template', $.identifier),
        $.expression_argument_list,
        'as',
        field('name', $.identifier),
        ';',
      ),

    // --- Piece level (Parser::piece_decl) -------------------------------

    piece_declaration: ($) =>
      seq(
        'piece',
        // A template's piece is named twice: once as the template, in code,
        // and once as the piece, on the page.
        optional(seq(field('template_name', $.identifier), $.parameter_list)),
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
            $.let_declaration,
            $.function_declaration,
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
            $.let_declaration,
            $.function_declaration,
            $.performance_declaration,
            $.studio_declaration,
            $.signature_declaration,
            $.structure_declaration,
            $.template_declaration,
            $.make_statement,
          ),
        ),
        '}',
      ),

    // Parser::import_stmt — `import "../library/motifs.musa";` or
    // `import std::tonal::harmony;`. How many segments a path has is a fact
    // about the package it names, so nothing here counts them.
    import_statement: ($) =>
      seq(
        'import',
        field('path', choice($.string, seq($.identifier, repeat1(seq(':', ':', $._module_name))))),
        optional(seq('as', field('alias', $.identifier))),
        ';',
      ),

    // A module may be named after a domain keyword — `harmony`, `pitch`,
    // `scale` — and the lexer writes the keyword token wherever the word
    // appears (Parser::MODULE_NAME). `list` and `option` are ordinary
    // identifiers now that the types are `List` and `Option`.
    _module_name: ($) => choice($.identifier, 'harmony', 'pitch', 'scale'),

    // Parser::front_matter_stmt — one shape, four heads.
    front_matter_statement: ($) =>
      seq(choice('subtitle', 'composer', 'arranger', 'copyright'), field('value', $.string), ';'),

    // Parser::tempo_stmt — a metronome mark, a tempo word, or both; a
    // gradual change adds where it arrives and how far it takes to get there.
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
    // barlines from here. Any identifier parses; the compiler
    // checks the word, because there is nothing else `meter` can be
    // followed by. `none` is an identifier here and not the absent case of
    // an option, which is `None`.
    meter_statement: ($) => seq('meter', field('meter', choice($.rational, $.identifier)), ';'),

    // Parser::key_stmt — `key a minor;`, or `key k;` when the key is
    // already a value. One name and nothing after it is the value form; a
    // tonic is always followed by its mode.
    key_statement: ($) =>
      seq(
        'key',
        choice(
          seq(field('pitch_class', $.pitch_class), field('mode', $.identifier)),
          field('key', $.expression),
        ),
        ';',
      ),

    // Parser::in_scale_stmt — `in scale c dorian { ... }`, or `in scale s
    // { ... }` when the scale is already a value. The tonic-and-collection
    // form is written out here because `scale` is the statement's own
    // keyword, not the start of a nested expression.
    in_scale_statement: ($) =>
      seq('in', 'scale', field('scale', $._scale_context), field('body', $.block)),

    _scale_context: ($) =>
      choice(seq(field('tonic', $.pitch_class), field('collection', $.identifier)), $.expression),

    // Parser::pitch_class — `a`, `g#`, `bb`. A flat is part of the identifier
    // and a sharp is a token of its own, so a tonic is one token or three.
    pitch_class: ($) => seq($.identifier, repeat('#')),

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
        field('type', $.type_name),
        optional(seq('=', field('default', choice($.pitch_literal, $.rational, $.integer)))),
      ),

    // Parser::fragment_decl — a motif without parameters, tagged differently.
    fragment_declaration: ($) => seq('fragment', field('name', $.identifier), field('body', $.block)),

    // Parser::let_decl / fn_decl — declarations evaluate only at the
    // elaboration stage; the temporal kernel never sees these nodes.
    let_declaration: ($) =>
      seq('let', field('name', $.identifier), ':', field('type', $.type_expression), '=', field('value', $.expression), ';'),

    // Parser::fn_decl — the body is a block, like every other body in the
    // language: `fn f(x: nat) -> nat { g(x) }`. The `= expression;` form is
    // a syntax error in the hand parser, so not a clean parse here either.
    function_declaration: ($) =>
      seq(
        'fn',
        field('name', $.identifier),
        $.parameter_list,
        '->',
        field('result', $.type_expression),
        field('body', $.block_expression),
      ),

    parameter_list: ($) =>
      seq('(', optional(seq($.parameter, repeat(seq(',', $.parameter)), optional(','))), ')'),

    parameter: ($) =>
      seq(
        field('name', $.identifier),
        ':',
        field('type', $.type_expression),
        optional(seq('=', field('default', $.expression))),
      ),

    // Function arrows associate right. Parentheses group a single type and
    // a comma makes a product; option/list are the only type constructors.
    type_expression: ($) =>
      choice(
        prec.right(1, seq($._type_atom, '->', $.type_expression)),
        $._type_atom,
      ),

    _type_atom: ($) =>
      choice(
        $.type_name,
        $.option_type,
        $.list_type,
        seq('(', $.type_expression, ')'),
        $.product_type,
      ),

    // A type is spelled with a capital, so it is an identifier and no
    // keyword stands here: `key` is a statement and `Key` is a type
    // (Parser::type_atom).
    type_name: ($) => $.identifier,
    option_type: ($) => seq('Option', '<', $.type_expression, '>'),
    list_type: ($) => seq('List', '<', $.type_expression, '>'),
    product_type: ($) =>
      seq('(', $.type_expression, ',', $.type_expression, repeat(seq(',', $.type_expression)), ')'),

    // One ordinary call notation for values, folds, and music-producing
    // functions. Application binds tightest.
    expression: ($) =>
      choice(
        $.match_expression,
        $.music_expression,
        $.application_expression,
        $.pitch_expression,
        $.step_expression,
        $.scale_expression,
        $.chord_expression,
        $.key_expression,
        $._primary_expression,
      ),

    application_expression: ($) =>
      prec.left(2, seq($._primary_expression, repeat1($.expression_argument_list))),

    pitch_expression: ($) =>
      prec.left(
        1,
        seq(
          $._pitch_operand,
          field('direction', choice('up', 'down')),
          field('interval', $._pitch_operand),
        ),
      ),

    // Parser::expr — `c5 step 2`, `c5 step down 1`. It binds tighter than
    // `up`/`down`, because a step is a coordinate move and the interval move
    // is applied to whatever it lands on.
    step_expression: ($) =>
      prec.left(
        2,
        seq($._pitch_operand, 'step', optional(field('direction', choice('up', 'down'))), field('steps', $._step_count)),
      ),

    // How many steps: a count, not a pitch, so this is its own operand.
    _step_count: ($) => choice($.integer, $.identifier, seq('(', $.expression, ')')),

    // Parser::scale_expr — `scale c dorian`.
    scale_expression: ($) =>
      seq('scale', field('tonic', $.pitch_class), field('collection', $.identifier)),

    // Parser::chord_expr — `chord c major7`. Rooted spelled content, with no
    // register: the same shape as `scale c dorian`, because the words naming
    // a chord type are a closed vocabulary rather than a value anyone writes.
    chord_expression: ($) =>
      seq('chord', field('root', $.pitch_class), field('chord_type', $.identifier)),

    // Parser::key_expr — `key c minor` where a value, not a statement, is
    // wanted.
    key_expression: ($) => seq('key', field('tonic', $.pitch_class), field('mode', $.identifier)),

    _pitch_operand: ($) =>
      choice(
        $.pitch_literal,
        $.interval_literal,
        $.identifier,
        seq('(', $.expression, ')'),
      ),

    expression_argument_list: ($) =>
      seq('(', optional(seq($.expression_argument, repeat(seq(',', $.expression_argument)), optional(','))), ')'),

    expression_argument: ($) =>
      seq(optional(seq(field('name', $.identifier), ':')), $.expression),

    _primary_expression: ($) =>
      choice(
        $.name_expression,
        $.literal_expression,
        $.option_expression,
        $.list_expression,
        $.product_expression,
        $.block_expression,
        seq('(', $.expression, ')'),
      ),

    // Parser::block_expr — `{ expression }`. A block is a
    // delimiter, not a sequence: it holds exactly one expression and means
    // exactly that expression.
    block_expression: ($) => seq('{', $.expression, '}'),

    // `repeat` is both the notation statement and the compiler-owned finite
    // value operation; expression position disambiguates it without making
    // the keyword a general identifier.
    // `M.member` is one name written in two words, which is why the dot is
    // part of the name rather than an operator over two of them.
    name_expression: ($) =>
      choice(
        seq($.identifier, optional(seq('.', field('member', $.identifier)))),
        'repeat',
        'transpose',
        'stretch',
        'retrograde',
        'invert',
      ),
    literal_expression: ($) => choice($.integer, $.rational, $.pitch_literal, $.interval_literal, 'true', 'false'),
    option_expression: ($) => choice('None', seq('Some', '(', $.expression, ')')),
    list_expression: ($) => seq('[', optional(seq($.expression, repeat(seq(',', $.expression)))), ']'),
    product_expression: ($) =>
      seq('(', $.expression, ',', $.expression, repeat(seq(',', $.expression)), ')'),

    match_expression: ($) =>
      seq(
        'match',
        field('value', $.expression),
        '{',
        $.match_arm,
        repeat(seq(',', $.match_arm)),
        optional(','),
        '}',
      ),

    match_arm: ($) => seq(field('pattern', $.pattern), '->', field('value', $.expression)),

    pattern: ($) =>
      choice(
        $.identifier,
        $.integer,
        $.rational,
        $.pitch_literal,
        $.interval_literal,
        'true',
        'false',
        'None',
        seq('Some', '(', $.identifier, ')'),
        seq('[', ']'),
        seq('[', $.identifier, ',', '.', '.', $.identifier, ']'),
        seq('(', $.identifier, ',', $.identifier, repeat(seq(',', $.identifier)), ')'),
      ),

    music_expression: ($) => seq('music', '{', repeat(choice(...VOICE_ITEMS($))), '}'),

    // --- Score level (Parser::score_decl and below) ---------------------

    score_declaration: ($) =>
      seq(
        'score',
        '{',
        repeat(choice($.part_declaration, $.section_statement, $.harmony_declaration)),
        '}',
      ),

    // Parser::part_decl — a part carries its own clef, and may carry its
    // own meter and tempo: polymeter and polytempo.
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
            $.make_statement,
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
      seq(
        'voice',
        field('name', $.identifier),
        optional($.parameter_list),
        '{',
        repeat(choice(...VOICE_ITEMS($))),
        '}',
      ),

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
    //
    // `scale` is a processor here and a musical collection everywhere else,
    // so the lexer writes the keyword token and this position accepts it
    // (`Parser::stage`). Only this position: a nested construction inside an
    // argument is identifiers alone, as `Parser::value` is, which is why the
    // keyword is added by aliasing rather than by widening the shared rules.
    stage: ($) => choice(alias($._stage_call, $.call_expression), alias($._stage_name, $.name_reference)),

    _stage_call: ($) => seq(field('name', choice($.identifier, 'scale')), $.argument_list),

    _stage_name: ($) => choice('output', 'master', $.identifier, 'scale'),

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

    // Parser::note_stmt — `<pitch-or-ref> <duration> <articulation>* ~?`.
    // No terminator: an event is self-delimiting.
    note_statement: ($) =>
      seq(
        field(
          'pitch',
          choice(
            $.pitch_literal,
            $.identifier,
            $.pitch_expression,
            $.step_expression,
            seq('(', $.expression, ')'),
          ),
        ),
        $.duration,
        optional($.articulation_list),
        optional('~'),
      ),

    // Parser::articulations — their own node, by the hand parser's own rule:
    // a bare identifier here is an articulation, not a reference. `>` is an
    // accent and `^` a marcato, which is the mark notation draws.
    articulation_list: ($) => repeat1(choice($.identifier, '>', '^')),

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

    rest_statement: ($) => seq('rest', $.duration),

    // Parser::stack_stmt — `stack c4 major7/2`, the close-position sugar. A
    // pitch class parses here and the compiler refuses it, so the mistake is
    // answered with a sentence about register rather than a parse error.
    stack_statement: ($) =>
      seq(
        'stack',
        field('root', choice($.pitch_literal, $.pitch_class)),
        field('chord_type', $.identifier),
        $.duration,
        optional($.articulation_list),
        optional('~'),
      ),

    // Parser::chord_stmt — `[<pitch> ...]<duration>`. The bracket says chord,
    // so the keyword and the commas were both repeating it.
    chord_statement: ($) =>
      seq(
        '[',
        repeat1($.pitch_literal),
        ']',
        $.duration,
        optional($.articulation_list),
        optional('~'),
      ),

    // Parser::use_stmt — an ordinary expression expected to have type music;
    // `with` remains the legacy occurrence-specialization suffix.
    use_statement: ($) =>
      seq(
        'use',
        field('value', $.expression),
        choice(field('overrides', $.with_clause), ';'),
      ),

    // Parser::with_clause — `with { note <n> = <pitch>; ... }`
    with_clause: ($) => seq('with', '{', repeat($.override_statement), '}'),

    override_statement: ($) =>
      seq('note', field('index', $.integer), '=', field('pitch', $.pitch_literal), ';'),

    // Parser::transpose_stmt — `transpose up|down <interval> { ... }`
    transpose_statement: ($) =>
      prec(3, seq('transpose', field('direction', choice('up', 'down')), field('interval', $.interval_literal), field('body', $.block))),

    // Parser::repeat_stmt — a count, or a range the realization chooses in.
    repeat_statement: ($) =>
      seq('repeat', field('count', $.integer), optional(seq('to', field('maximum', $.integer))), field('body', $.block)),

    // Parser::ending_stmt — `ending 1 { ... }`
    ending_statement: ($) => seq('ending', field('pass', $.integer), field('body', $.block)),

    // Parser::bar_stmt and Parser::pipe_bar_stmt — one node for both, because
    // it is one claim: a measure's worth of music. The named form keeps its
    // block, because a name is an address; the drawn form runs to the next
    // `|` or to the first statement that is itself at least a bar long.
    bar_statement: ($) =>
      choice(
        seq('bar', optional(field('name', $.identifier)), field('body', $.block)),
        // Right-associative: a bar takes every item it can, and the next
        // `|` is what stops it — the hand parser's exit-before-dispatch.
        prec.right(seq('|', repeat(choice(...BAR_ITEMS($))))),
      ),

    // Parser::assert_stmt — `assert pitches_in(scale c major) { ... }`. The
    // parentheses are written even when the claim takes no arguments, so that
    // the grammar tells `assert fills_meter()` apart from a name someone
    // misremembered. Which names are claims is the compiler's registry, not
    // the grammar's: the shape is here, the vocabulary is there.
    assert_statement: ($) =>
      seq(
        'assert',
        field('claim', $.identifier),
        field('arguments', $.expression_argument_list),
        field('body', $.block),
      ),

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

    // A grace note ends itself, the way every other event does; the group's
    // `}` ends the last one.
    grace_note: ($) => seq(field('pitch', choice($.pitch_literal, $.identifier)), optional($.articulation_list)),

    // Parser::phrase_stmt — `phrase "A" { ... }`
    phrase_statement: ($) => seq('phrase', field('name', $.string), field('body', $.block)),

    // Parser::hairpin_stmt — `crescendo to f { ... }`
    hairpin_statement: ($) =>
      seq(choice('crescendo', 'diminuendo'), 'to', field('dynamic', $.identifier), field('body', $.block)),

    // Parser::dynamic_stmt — `dynamic <mark>;`
    dynamic_statement: ($) => seq('dynamic', field('mark', $.identifier), ';'),

    // Parser::senza_stmt — `senza { ... }`: the barlines stop for exactly as
    // long as the block, then the meter returns. It is the two
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

    // Parser::chord_symbol — `am`, `fmaj7`, `f#m7`. One word, however it
    // lexes: a sharp on the root splits the quality off into its own
    // identifier.
    chord_symbol: ($) =>
      seq(
        choice($.identifier, $.pitch_literal),
        repeat('#'),
        optional($.identifier),
        optional($.integer),
      ),

    // --- Tokens (crates/musa-language/src/lexer.rs) ----------------------

    // `[a-g](#+|b+|n)?-?[0-9]+` — a written pitch: letter, accidental,
    // octave. The letter is always first, so the `b` of `bb2` is a flat and
    // the `b` of `b2` is the note.
    pitch_literal: ($) => /[a-g](#+|b+|n)?-?[0-9]+/,

    // `d4` is the pitch D4, so a singly diminished interval is `dim4`;
    // repeated diminution remains compact (`dd4`, `ddd4`).
    interval_literal: ($) => /(P|M|m|A+|d{2,}|dim)[0-9]+/,

    rational: ($) => /[0-9]+\/[0-9]+/,
    float: ($) => /[0-9]+\.[0-9]+/,
    integer: ($) => /[0-9]+/,
    identifier: ($) => /[a-zA-Z_][a-zA-Z_0-9]*/,

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
