use crate::diagnose::Code;
use crate::origin::SourceSpan;

/// One syntax import from the fixed module header.
pub(crate) struct SyntaxImport {
    pub(crate) alias: String,
    pub(crate) path: String,
    /// Where the statement ends, which is what "before any definition that
    /// uses it" is checked against.
    pub(crate) ends: u32,
    /// The whole statement, which is where an adapter that promises more than
    /// it offers is refused.
    pub(crate) at: SourceSpan,
}

/// What an adapter promises, in `26-language-design-decision.md` §4's words.
///
/// Three levels and an order on them, because each one is the one below it plus
/// an operation: *readable* expands, *editable* also edits under the edit law,
/// *generative* also prints under the round-trip law. A level is what a
/// musician is told about a region — whether a control is greyed, whether an
/// interface may offer to make one — so it is declared by the adapter and
/// checked against what the module actually holds, rather than inferred from
/// which `let`s happen to be there. Inferring it would make adding a
/// half-finished `edit` silently promise the edit law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    /// Expansion only: structured views of its regions are read-only.
    Readable,
    /// Expansion and `edit`, under the edit law.
    Editable,
    /// Editable, and `print` for new regions, under the round-trip law.
    Generative,
}

impl Level {
    /// The level a module declares, as it spells it.
    fn named(word: &str) -> Option<Self> {
        match word {
            "readable" => Some(Self::Readable),
            "editable" => Some(Self::Editable),
            "generative" => Some(Self::Generative),
            _ => None,
        }
    }

    /// The word for it, for a sentence a reader gets.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Readable => "readable",
            Self::Editable => "editable",
            Self::Generative => "generative",
        }
    }

    /// Everything a module at this level must declare.
    fn operations(self) -> &'static [&'static str] {
        match self {
            Self::Readable => &["expand"],
            Self::Editable => &["expand", "edit"],
            Self::Generative => &["expand", "edit", "print"],
        }
    }
}

/// Why a module could not be read as an adapter at the level it claims.
///
/// A message and a help, and no span: the same fault is reported at the import
/// that names the module and at a region the module reads, and which of those
/// the caret belongs on is the caller's question rather than this one's.
///
/// And causes, for the other half of the same argument. Which of the caller's
/// spans is right is the caller's question; which document the module's own
/// faults are in is not a question at all, so they travel whole rather than
/// being restated anywhere.
pub(crate) struct LevelFault {
    pub(crate) message: String,
    pub(crate) help: &'static str,
    /// Which complaint this is. A module that crossed a compilation limit
    /// while being read is a limit and says so, exactly as a stop during
    /// expansion does; everything else here is the module's own fault.
    pub(crate) code: Code,
    /// What the module's own checker said, when it said anything.
    pub(crate) causes: Vec<crate::diagnose::Cause>,
}

/// The level `adapter_source` declares, checked against what it offers.
///
/// The check `26-language-design-decision.md` §4 asks for: a declared level is
/// a promise, and a module that promises `generative` while declaring no
/// `print` has promised something no musician can rely on. Under-promising is
/// allowed and over-promising is not — a module may hold an `edit` it does not
/// advertise, and what governs is the word it wrote.
///
/// A module that declares no level at all is refused rather than defaulted.
/// "Declared and checked, not inferred" is the whole point: a default would be
/// the compiler deciding what a package promises.
///
/// `path` is the module as the importer wrote it, which is what the messages
/// name it by; `document` is the key the import resolved to, which is what a
/// cause is filed under, what a renderer looks the text up with, and what the
/// module's own imports resolve against.
pub(crate) fn level_of(
    adapter_source: &str,
    path: &str,
    document: &str,
    sources: &crate::imports::ImportSources,
) -> Result<Level, LevelFault> {
    let plain = |message: String, help: &'static str| LevelFault {
        message,
        help,
        code: Code::Expansion,
        causes: Vec::new(),
    };
    let module = crate::core::read_adapter_module(adapter_source, crate::core::PhaseImports::at(document, sources))
        .map_err(|fault| match fault {
            crate::core::ModuleFault::Stopped => LevelFault {
                message: format!("reading `{path}` crossed a compilation limit"),
                help: "an adapter is total, so this is a limit rather than a loop",
                code: Code::ResourceLimit,
                // A read that ran out of budget said nothing about the module, so
                // there is nothing to carry.
                causes: Vec::new(),
            },
            // The wrapper's own sentence, and not a word of the module's spliced
            // into it: the causes below say what is wrong inside the module, each
            // at its own place in it, and a summary here would say the first one
            // twice and the rest not at all.
            crate::core::ModuleFault::Broken(diagnostics) => LevelFault {
                message: format!("`{path}` is not an adapter module"),
                help: "an adapter module is a `library` of ordinary declarations, checked in the expansion phase",
                code: Code::Expansion,
                causes: diagnostics
                    .into_iter()
                    .map(|diagnostic| crate::diagnose::Cause::of(document, diagnostic))
                    .collect(),
            },
        })?;
    let declared = module.text("level").ok_or_else(|| {
        plain(
            format!("`{path}` declares no conformance level"),
            "an adapter module declares `let level = \"readable\";`, `\"editable\"`, or `\"generative\"`",
        )
    })?;
    let level = Level::named(&declared).ok_or_else(|| {
        plain(
            format!("`{path}` declares the level `{declared}`, which is not one of the three"),
            "the levels are `readable`, `editable`, and `generative`, and each is the one before it plus an operation",
        )
    })?;
    for operation in level.operations() {
        if !module.declares(operation) {
            return Err(plain(
                format!("`{path}` declares the {} level and no `{operation}`", level.word()),
                "a level is a promise: declare the level the module reaches, or write the operation it names",
            ));
        }
    }
    Ok(level)
}
