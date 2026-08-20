//! What a machine value is, stated as source programs.
//!
//! `docs/rules/across-stages/03-machine-calculus.md` §2 admits eight forms and
//! no others: a registered primitive, `identity`, `connect`, `beside`,
//! `feedback`, `copy`, `drop`, and `swap`. Every one of them is written here in
//! a complete piece, because a typing rule that has never been written down in
//! the language it governs is a rule about a language nobody has.
//!
//! The other half is what the calculus *refuses*: a hidden closure in a port or
//! a feedback value, a port that is not storable data, a step written as
//! something that is not a step, a `feedback` with no initial value, and a
//! `lift` from a source function. Each is refused here by name.
//!
//! One thing §2 refuses cannot be stated from outside the crate: two machines
//! whose steps count different things do not connect. The governing grammar
//! names exactly one step tag, so a second one exists only under `cfg(test)`
//! inside `musa-compiler`, and that law is stated there — an integration test
//! links the ordinary build, where every registered unit counts frames and the
//! law would pass without being asked.

// A law suite reports a violated law by failing, and the helpers below take
// apart a projection whose existence the law has already asserted: a panic is
// the report, not an accident.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{Code, Compilation, CompileOptions, SourceDocument, compile};

fn compile_machines(declarations: &str) -> Compilation {
    let source = SourceDocument::new(
        format!("piece \"Machines\" {{ {declarations} score {{ part p {{ voice v {{ c4/1 }} }} }} }}"),
        "machine-laws.musa",
    );
    compile(&source, &CompileOptions::default())
}

fn errors(compilation: &Compilation) -> Vec<Code> {
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// What a rejected program was told, so a test can say the composer was told
/// the right thing rather than merely refused.
fn complaint(compilation: &Compilation) -> String {
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_compiler::Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}

fn machine<'a>(compilation: &'a Compilation, name: &str) -> &'a musa_compiler::MachineSpec {
    match compilation.machine(name) {
        Some(machine) => machine,
        None => panic!("`{name}` did not project a machine: {:?}", compilation.diagnostics()),
    }
}

/// §2's `machine(p)`: a registered unit, instantiated at a written name and
/// version, becomes a machine at the ports the registry declares.
///
/// The ports are the *unit's*, not the call's: nothing in the source says
/// `Ratio`, and both ends of the projection say it anyway.
#[test]
fn a_registered_unit_becomes_a_machine_at_the_ports_it_declares() {
    let compiled = compile_machines("let m = machine(primitive(\"scale\", 1, 3/2));");
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    let projected = machine(&compiled, "m");
    assert_eq!(projected.step(), "AudioFrameStep");
    assert_eq!(projected.input(), "Ratio");
    assert_eq!(projected.output(), "Ratio");
    assert_eq!(projected.nodes().len(), 1);
    assert_eq!(
        projected.nodes().first().map(musa_compiler::SpecNode::form),
        Some(musa_compiler::SpecForm::Primitive)
    );
    assert_eq!(
        projected.nodes().first().and_then(musa_compiler::SpecNode::id),
        Some("scale")
    );
}

/// §2's `connect(m, n)`: a chain, where the first machine's output is the
/// second's input.
///
/// The two units are two *versions* of one name, which is the registry rule
/// doing visible work: version 1 and version 2 configure differently, and both
/// are `scale`.
#[test]
fn a_chain_types_at_the_ends_and_keeps_the_middle_private() {
    let compiled = compile_machines(
        "let m = connect(machine(primitive(\"scale\", 1, 3/2)), machine(primitive(\"scale\", 2, (2/1, 1/1))));",
    );
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    let projected = machine(&compiled, "m");
    assert_eq!((projected.input(), projected.output()), ("Ratio", "Ratio"));
    assert_eq!(projected.nodes().len(), 3, "two units and the chain that joins them");
    let root = projected.root().expect("a projection has a root");
    assert_eq!(
        projected.nodes().get(root).map(musa_compiler::SpecNode::form),
        Some(musa_compiler::SpecForm::Connect)
    );
    for child in projected
        .nodes()
        .get(root)
        .map(musa_compiler::SpecNode::children)
        .unwrap_or_default()
    {
        assert!(
            *child < root,
            "a node's children precede it, so one walk forwards suffices"
        );
    }
}

/// A chain whose ends do not meet is refused, and the complaint names the two
/// port types rather than the whole expression.
#[test]
fn a_chain_whose_ends_do_not_meet_is_refused() {
    let compiled = compile_machines(
        "let m = connect(machine(primitive(\"reached\", 1, 4)), machine(primitive(\"scale\", 1, 3/2)));",
    );
    assert_eq!(errors(&compiled), vec![Code::ConversionMismatch]);
    assert!(
        complaint(&compiled).contains("Bool"),
        "the complaint names the port that did not meet: {}",
        complaint(&compiled)
    );
}

/// §2's `beside(m, n)`: two paths side by side, on a pair.
///
/// `beside` keeps two outputs. Mixing them is a *primitive* — §2 says so in as
/// many words — which is why the third program below has to name one.
#[test]
fn side_by_side_paths_carry_a_pair_through_and_keep_both_outputs() {
    let compiled = compile_machines(
        "let m = beside(machine(primitive(\"scale\", 1, 3/2)), machine(primitive(\"scale\", 1, 2/1)));",
    );
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    let projected = machine(&compiled, "m");
    assert_eq!(projected.input(), "(Ratio, Ratio)");
    assert_eq!(projected.output(), "(Ratio, Ratio)", "beside keeps two outputs");
    assert_eq!(projected.nodes().len(), 3);
}

/// A complete small piece of wiring: one path is scaled, the other passes
/// through, and a registered mixer brings them back to one.
#[test]
fn a_scaled_path_and_a_dry_path_meet_at_a_registered_mixer() {
    let compiled = compile_machines(
        "let m = connect(beside(machine(primitive(\"scale\", 1, 3/2)), identity), \
         machine(primitive(\"mix\", 1, (1/1, 1/1))));",
    );
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    let projected = machine(&compiled, "m");
    assert_eq!((projected.input(), projected.output()), ("(Ratio, Ratio)", "Ratio"));
    assert_eq!(projected.nodes().len(), 5);
}

/// §2's `feedback(initial, m)`: the stored value the first step reads, and the
/// machine it is fed back through.
#[test]
fn initialized_feedback_hides_the_stored_port_from_the_outside() {
    let compiled =
        compile_machines("let m = feedback(0/1, connect(machine(primitive(\"mix\", 1, (1/1, 1/1))), copy));");
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    let projected = machine(&compiled, "m");
    assert_eq!(
        (projected.input(), projected.output()),
        ("Ratio", "Ratio"),
        "the stored value is the loop's, not the caller's"
    );
    let root = projected.root().expect("a projection has a root");
    let node = projected.nodes().get(root).expect("the root is a node");
    assert_eq!(node.form(), musa_compiler::SpecForm::Feedback);
    assert!(
        !node.stored().is_empty(),
        "the initial value is stored exactly, so the first step has something to read"
    );
}

/// There is no uninitialized feedback to write. `feedback` takes the value the
/// first step reads as an argument, so a loop with no delay is not a program
/// the checker has to reject — it is not a program that can be spelled.
#[test]
fn feedback_cannot_be_written_without_the_value_its_first_step_reads() {
    let compiled = compile_machines("let m = feedback(machine(primitive(\"scale\", 1, 3/2)));");
    assert_eq!(errors(&compiled), vec![Code::WrongArity]);
    assert!(
        complaint(&compiled).contains("takes 2 arguments"),
        "{}",
        complaint(&compiled)
    );
}

/// §2's `copy`, `drop`, and `swap`: wiring, not musical or audio operations.
/// Each is a machine, so each is written as a name rather than applied.
#[test]
fn the_three_wiring_machines_are_names_and_type_as_wiring() {
    let fanned = compile_machines("let m = connect(copy, machine(primitive(\"mix\", 1, (1/1, 1/1))));");
    assert!(errors(&fanned).is_empty(), "{}", complaint(&fanned));
    assert_eq!(
        (machine(&fanned, "m").input(), machine(&fanned, "m").output()),
        ("Ratio", "Ratio"),
        "`copy` splits one value into the pair a mixer takes"
    );

    let dropped = compile_machines("let m = connect(machine(primitive(\"scale\", 1, 3/2)), drop);");
    assert!(errors(&dropped).is_empty(), "{}", complaint(&dropped));
    assert_eq!(machine(&dropped, "m").output(), "Unit");

    let swapped = compile_machines("let m = connect(swap, machine(primitive(\"mix\", 1, (1/1, 1/1))));");
    assert!(errors(&swapped).is_empty(), "{}", complaint(&swapped));
    assert_eq!(machine(&swapped, "m").input(), "(Ratio, Ratio)");
}

/// `identity` is a machine at every step and every port, so written alone it
/// decides none of them, and the refusal arrives at the use that asks for a
/// port. Written with a type, it is a machine like any other.
///
/// The `let` itself is admitted under either checker: binding the polymorphic
/// value asks nothing, because the ports are quantified at the declaration and
/// solved at the use (`02-core-calculus.md` §2.1). What is refused is the use
/// that leaves them undetermined — and nothing can *project* a machine whose
/// ports were never written, which is the consumer's half of the law.
/// `document::laws::an_open_machine_is_refused_until_its_ports_are_written` is
/// the same law from the document's side, and the two agreeing is the point:
/// one program cannot be accepted by the elaborator and refused by the reading.
#[test]
fn an_undecided_machine_is_a_value_and_not_yet_a_projection() {
    let open = compile_machines("let m = identity;");
    assert!(
        errors(&open).is_empty(),
        "the polymorphic value binds: nothing was asked of it yet: {}",
        complaint(&open)
    );
    assert!(
        open.machine("m").is_none(),
        "a port whose type nothing decided is not a port a consumer can prepare"
    );
    let used = compile_machines("let m = identity; let n = connect(m, m);");
    assert!(
        complaint(&used).contains("could not determine"),
        "the use that cannot decide the ports is refused: {}",
        complaint(&used)
    );

    let decided = compile_machines("let m: Machine<AudioFrameStep, Ratio, Ratio> = identity;");
    assert!(errors(&decided).is_empty(), "{}", complaint(&decided));
    assert_eq!(machine(&decided, "m").input(), "Ratio");
    assert_eq!(decided.machine_names(), vec!["m"]);
}

/// Exact identity: two machines are one machine when their descriptions are,
/// and a configuration is part of the description rather than beside it.
#[test]
fn a_configuration_is_part_of_which_machine_this_is() {
    let compiled = compile_machines(
        "let a = machine(primitive(\"scale\", 1, 3/2)); \
         let b = machine(primitive(\"scale\", 1, 3/2)); \
         let c = machine(primitive(\"scale\", 1, 2/1));",
    );
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    assert_eq!(
        machine(&compiled, "a").digest(),
        machine(&compiled, "b").digest(),
        "one description, one identity"
    );
    assert_ne!(
        machine(&compiled, "a").digest(),
        machine(&compiled, "c").digest(),
        "a unit configured differently is a different machine"
    );
}

/// Wiring is not commutative, and the identity says so: the same two parts
/// joined the other way round are a different machine.
#[test]
fn the_order_of_a_chain_is_part_of_its_identity() {
    let compiled = compile_machines(
        "let m = connect(machine(primitive(\"scale\", 1, 3/2)), identity); \
         let n = connect(identity, machine(primitive(\"scale\", 1, 3/2)));",
    );
    assert!(errors(&compiled).is_empty(), "{}", complaint(&compiled));
    assert_ne!(machine(&compiled, "m").digest(), machine(&compiled, "n").digest());
}

/// §1.1: a machine's ports are storable data. An arrow is refused where the
/// port is written, and refused as `02-core-calculus.md` §1.2's `Storable`:
/// instances exist only on declared types and are generated, so a function type
/// has none and no author could write one.
///
/// *Both* ports hold the arrow, because `identity`'s two are one binder: a
/// program that wrote the arrow in one of them would disagree with itself about
/// that binder and be refused for the disagreement, one refusal short of the
/// one this law is about.
#[test]
fn a_port_that_holds_a_function_is_refused_where_it_is_written() {
    let compiled = compile_machines("let m: Machine<AudioFrameStep, Ratio -> Ratio, Ratio -> Ratio> = identity;");
    assert_eq!(errors(&compiled), vec![Code::TypeMismatch]);
    assert!(
        complaint(&compiled).contains("not storable data") && complaint(&compiled).contains("Ratio → Ratio"),
        "the complaint names the type that cannot be stored: {}",
        complaint(&compiled)
    );
}

/// The same rule reached the other way: a feedback value is storable data, so a
/// closure cannot be smuggled around the loop. Nothing checks for a closure —
/// `feedback`'s signature requires `Storable` of the value it stores, and an
/// arrow is the one type §1.2 says can never have it.
#[test]
fn a_closure_cannot_be_carried_through_a_feedback_loop() {
    let compiled = compile_machines("let m = feedback(fn (x: Ratio) -> Ratio { x }, identity);");
    assert_eq!(errors(&compiled), vec![Code::TypeMismatch]);
    assert!(
        complaint(&compiled).contains("not storable data") && complaint(&compiled).contains("Ratio → Ratio"),
        "{}",
        complaint(&compiled)
    );
}

/// A step says what one step *counts*. Something that is not a step is refused
/// at the one place a step can be written.
///
/// Refused by the *reading* and not by the signature, which is the one premise
/// of §2 that could not become a constraint: a step tag is a host notion, the
/// build's registry owns the list of them, and `Machine Nat A B` is a perfectly
/// well-typed core term. So the position is restricted where the position is.
#[test]
fn a_step_position_takes_a_step_and_nothing_else() {
    let compiled = compile_machines("let m: Machine<Nat, Ratio, Ratio> = identity;");
    assert_eq!(errors(&compiled), vec![Code::WrongArity]);
    assert!(complaint(&compiled).contains("not a step"), "{}", complaint(&compiled));
}

/// §2: "There is no public `lift` from a source function into a machine." Not a
/// refusal with a special message — the name simply does not exist, which is
/// the strongest form the statement can take.
#[test]
fn there_is_no_lift_from_a_source_function() {
    let compiled = compile_machines("let m = lift(fn (x: Ratio) -> Ratio { x });");
    assert_eq!(errors(&compiled), vec![Code::UnknownName]);
    assert!(complaint(&compiled).contains("lift"), "{}", complaint(&compiled));
}

/// §1: a pair `(name, version)` selects exactly one unit, so both have to be
/// written out. A version the piece computes is a unit the compiler cannot type
/// the ports of.
#[test]
fn a_registered_unit_is_named_by_a_written_name_and_version() {
    let computed = compile_machines("let n = 1; let m = primitive(\"scale\", n, 3/2);");
    assert_eq!(errors(&computed), vec![Code::NotAValue]);
    assert!(
        complaint(&computed).contains("written name and version"),
        "{}",
        complaint(&computed)
    );

    // `UnknownWord` and not `UnknownName`: a unit id is not a name any scope
    // could have bound, it is a word out of a closed vocabulary this build
    // registers, and the report says which words are in it.
    let unknown = compile_machines("let m = primitive(\"nope\", 1, 3/2);");
    assert_eq!(errors(&unknown), vec![Code::UnknownWord]);

    let wrong_version = compile_machines("let m = primitive(\"scale\", 7, 3/2);");
    assert_eq!(errors(&wrong_version), vec![Code::UnknownWord]);
    assert!(
        complaint(&wrong_version).contains("no version 7 of `scale`"),
        "{}",
        complaint(&wrong_version)
    );
}

/// A configuration is checked against the shape the registry declares, which is
/// what makes a version a real distinction rather than a label.
#[test]
fn a_configuration_is_checked_against_the_version_it_configures() {
    let wrong_shape = compile_machines("let m = primitive(\"scale\", 1, true);");
    assert_eq!(errors(&wrong_shape), vec![Code::ConversionMismatch]);

    let other_version = compile_machines("let m = primitive(\"scale\", 2, 3/2);");
    assert_eq!(
        errors(&other_version),
        vec![Code::ConversionMismatch],
        "version 2 takes a pair, and version 1's configuration is not one"
    );

    let right_shape = compile_machines("let m = machine(primitive(\"scale\", 2, (3/2, 1/1)));");
    assert!(errors(&right_shape).is_empty(), "{}", complaint(&right_shape));
}

/// A registered unit is not a machine until `machine(p)` says so, and a machine
/// is never a registered unit. Two type formers, not one.
///
/// The complaint spells them the *core*'s way — `Primitive ?0 ?1 ?2`, an
/// application — because that is what the term is, and this crate has never
/// heard of the angle brackets the surface writes.
#[test]
fn a_machine_and_a_registered_unit_are_two_types() {
    let compiled = compile_machines("let m = machine(identity);");
    assert_eq!(errors(&compiled), vec![Code::ConversionMismatch]);
    assert!(
        complaint(&compiled).contains("Primitive ") && complaint(&compiled).contains("Machine "),
        "the complaint names both: {}",
        complaint(&compiled)
    );
}

/// Building a machine does not step it, and a machine is not a musical fact.
///
/// The two pieces below differ in exactly one thing — which unit the machine
/// is configured with — and are the same music. The machines are *not* the
/// same machine, which is the other half: a description that no listener can
/// hear still has an identity of its own, and the two identities answer
/// different questions.
///
/// The configurations are written the same width on purpose. A piece's
/// identity covers its provenance, so a longer declaration would move the
/// score's spans and the test would pass or fail for a reason that has nothing
/// to do with machines.
#[test]
fn a_machine_is_a_description_and_not_a_musical_fact() {
    let one = compile_machines("let m = machine(primitive(\"scale\", 1, 3/2));");
    let other = compile_machines("let m = machine(primitive(\"scale\", 1, 2/1));");
    assert!(errors(&one).is_empty(), "{}", complaint(&one));
    assert!(errors(&other).is_empty(), "{}", complaint(&other));
    assert_eq!(
        one.identity(),
        other.identity(),
        "a machine is a description, so configuring one differently changes no musical fact"
    );
    assert_ne!(machine(&one, "m").digest(), machine(&other, "m").digest());
}
