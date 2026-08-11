//! The kernel interchange format (docs/kernel/01-grammar.md): terms as text,
//! in both directions.
//!
//! This is what makes the kernel an *interchange* format rather than a dump.
//! A canon prints as a `let` and two `shift`s, not as a thousand occurrences,
//! because a term can say "this is that material again" and a value cannot.
//!
//! **Payloads stay opaque.** The kernel is generic in `A` and a file is not,
//! so the format carries a payload as a quoted string and hands it to the
//! caller's [`TextPayload`] implementation. The alternative — a payload
//! grammar in the kernel — would make this crate know what a note is, which
//! is the §12 invariant the crate exists to hold. The cost is one
//! indirection; the alternative is the kernel learning music theory.
//!
//! Indentation arithmetic and byte-offset advances are total on the sizes a
//! kernel file can have; the workspace arithmetic lint is allowed
//! module-wide (the sanctioned pattern, see musa-kernel/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use num_rational::Ratio;

use crate::error::KernelError;
use crate::occurrence::Occurrence;
use crate::term::{Form, Term};
use crate::time::{Beat, Span};
use crate::timeline::timeline;

/// How a payload is spelled in a file.
///
/// The two directions are one trait because they are one decision: a payload
/// text form that cannot be read back is not a text form, and N3 already
/// requires the writer to be injective, which is exactly the round-trip
/// property. Implementations live with the payload — `ScoreFact`'s is in
/// `musa-compiler` — and the kernel never looks inside the string.
pub trait PayloadText: Sized {
    /// The payload's text form. Must be injective on values (N3).
    fn to_text(&self) -> String;

    /// The inverse of [`Self::to_text`], or `None` if `text` is not one.
    fn from_text(text: &str) -> Option<Self>;
}

/// A payload that also *names* itself at the interchange boundary.
///
/// Spelling and naming are separate traits because a reader needs them at
/// separate times: a file announces its payload type in the header and only
/// then can anyone decide whether they know how to decode it. [`read`] accepts
/// a file knowing only [`PayloadText`]-worth about it — nothing — and the name
/// it recovers is what a caller matches against its own [`Self::type_name`].
/// Folding the two together would mean a file could not be read until it was
/// already understood, which is exactly backwards for an interchange format.
pub trait TextPayload: PayloadText {
    /// The payload type's name, as the file's `Timeline[…]` annotation.
    fn type_name() -> &'static str;
}

/// A payload carried verbatim, never decoded.
///
/// This is what a reader holds when it has parsed a file whose payload type it
/// does not implement: the syntax is known to be well-formed, the term
/// structure is known exactly, and the payload is a string the kernel has
/// (as always) not looked inside. A tool can format such a file, count its
/// occurrences, and check that it binds no free variable — everything except
/// say what it *means*, which is the one thing that needs the payload type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opaque(String);

impl Opaque {
    /// The payload's text, exactly as the file spelled it.
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl PayloadText for Opaque {
    fn to_text(&self) -> String {
        self.0.clone()
    }

    fn from_text(text: &str) -> Option<Self> {
        Some(Self(text.to_owned()))
    }
}

/// A kernel file, read.
///
/// One type for both readers, because a file is the same file either way: what
/// differs is only whether its payloads were decoded. [`parse`] fixes `A` to a
/// payload type the caller implements; [`read`] leaves it [`Opaque`], which is
/// what the toolchain needs to accept a `.musa.kernel` file as a document —
/// open it, format it, report its syntax errors — before it can know whether
/// the payload type is one this build supports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document<A> {
    name: String,
    payload_type: String,
    notes: Vec<String>,
    term: Term<A>,
}

impl<A> Document<A> {
    /// The name the file's `kernel "…"` header declares.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The payload type the composition is annotated with.
    ///
    /// A string rather than an enumeration: the set of payload types is open,
    /// and a kernel that enumerated them would be a kernel that knows what
    /// music is (§12). A caller compares this with its own
    /// [`TextPayload::type_name`] and decides.
    pub fn payload_type(&self) -> &str {
        &self.payload_type
    }

    /// The `%` note lines, in the order they were written, without the `%`.
    ///
    /// The kernel does not know what any of them mean — a note is where a
    /// producer records the reading of the work a file projects
    /// (`docs/kernel/11-realization.md`), and carrying the sentence is the
    /// whole of the kernel's involvement.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// The term, as written: `let`, `sequence`, `overlay`, `shift`, `scale`
    /// and `restrict` structure survives reading.
    pub fn term(&self) -> &Term<A> {
        &self.term
    }

    /// The term, taken, when the caller wants it and not the file around it.
    pub fn into_term(self) -> Term<A> {
        self.term
    }
}

impl<A: PayloadText> Document<A> {
    /// The file, printed canonically.
    ///
    /// The same writer [`print`] uses, so a formatted file and a generated one
    /// are byte-identical when they say the same thing — which is what makes
    /// "format" a well-defined operation on a document whose payloads this
    /// build cannot read.
    pub fn to_text(&self) -> String {
        write_document(&self.name, &self.payload_type, &self.term, &self.notes)
    }
}

/// The format version written into every file's header.
///
/// A string, not a negotiation scheme: a reader that does not recognise it
/// should say so and stop, which is all a version needs to do here.
pub const FORMAT_VERSION: &str = "musa-kernel-1";

/// Print `term` as a kernel file named `name`, with `notes` recorded above it.
///
/// The term is printed as written: `let`, `seq`, `over`, `shift` and `scale`
/// structure survives. Normalizing first is the caller's separate decision —
/// there is no "normalize while printing" flag, because that would complect
/// two choices a caller can make in sequence.
///
/// A note is one `%` line under the version, and the kernel never looks
/// inside it. It exists because a file is the projection of *one* reading of a
/// work and has to say which — a realization, in
/// `docs/kernel/11-realization.md`'s sense — and because the kernel must not
/// learn what a realization is to carry the sentence. Read back with
/// [`notes`]. Newlines are stripped, since a note that spanned two lines would
/// read back as two.
pub fn print<A: TextPayload>(name: &str, term: &Term<A>, notes: &[String]) -> String {
    write_document(name, A::type_name(), term, notes)
}

/// The one writer, with the payload type supplied rather than looked up.
///
/// [`print`] takes it from the type and [`Document::to_text`] takes it from
/// the file. There is deliberately no third source: a formatter that invented
/// an annotation would be rewriting the document it was asked to tidy.
fn write_document<A: PayloadText>(name: &str, payload_type: &str, term: &Term<A>, notes: &[String]) -> String {
    let mut out = String::with_capacity(256);
    let _ = writeln!(out, "% {FORMAT_VERSION}");
    for note in notes {
        let _ = writeln!(out, "% {}", note.replace(['\n', '\r'], " "));
    }
    let _ = write!(out, "kernel ");
    write_string(&mut out, name);
    let _ = writeln!(out, " {{");
    let _ = write!(out, "  composition main : Timeline[{payload_type}] =\n    ");
    write_term(&mut out, term, 2);
    let _ = writeln!(out, ";");
    let _ = writeln!(out, "}}");
    out
}

/// The note lines a file carries, in the order they were written.
///
/// Everything after the version line and before the term, without the `%` and
/// the space. A file with nothing to declare yields nothing, which is why this
/// is an iterator rather than a struct with optional fields: the kernel does
/// not know what any note means, and a shape would be a claim that it does.
pub fn notes(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .skip(1)
        .map_while(|line| line.trim_end().strip_prefix('%'))
        .map(|note| note.trim())
}

/// Parse a kernel file whose payloads are `A`.
///
/// Refuses a file annotated with any other payload type, which is what makes
/// this the *typed* reader: a caller that names `A` is saying it knows what
/// the occurrences mean, and a file that carries something else does not mean
/// it. Use [`read`] to accept the file first and decide afterwards.
///
/// The term is *not* checked: [`Term::check`] is the caller's next call, and
/// keeping the two apart means a parse error and a well-formedness error stay
/// distinguishable to whoever reports them.
///
/// # Errors
///
/// [`KernelError::Parse`], naming the byte offset and what was expected.
pub fn parse<A: TextPayload>(text: &str) -> Result<Document<A>, KernelError> {
    read_as::<A>(text, Some(A::type_name()))
}

/// Parse one composition expression: the right-hand side of a
/// `composition … =` and nothing around it.
///
/// The reader a *quotation* needs. A quote in a `.musa` file is one
/// composition expression — no version header, no `kernel "name" {`, no
/// declaration — and the host language must not grow a second reading of the
/// term grammar to accept it (`docs/language/01-surface.md` §7). So the host
/// hands the raw text here and gets back the same `Term<A>` a file would have
/// produced, with parse offsets counted from the start of `text`: the caller
/// knows where in its own document that was, and this does not.
///
/// The term is *not* checked, exactly as in [`parse`]: closure and shadowing
/// are the caller's next question, and a caller that binds holes around the
/// term must ask it *after* binding them, not before.
///
/// # Errors
///
/// [`KernelError::Parse`], naming the byte offset and what was expected.
pub fn parse_expression<A: TextPayload>(text: &str) -> Result<Term<A>, KernelError> {
    let mut cursor = Cursor::new(text);
    let term = cursor.term::<A>()?;
    cursor.end()?;
    Ok(term)
}

/// Read a kernel file without decoding its payloads.
///
/// Accepts every file [`parse`] accepts and more: a file whose payload type
/// this build has no implementation for is still a well-formed kernel file,
/// and refusing to read it would mean refusing to *show* it. What the caller
/// gets back names the payload type, so the decision about whether the file
/// can be evaluated is made where the payload types are known rather than
/// here.
///
/// # Errors
///
/// [`KernelError::Parse`], exactly as [`parse`] — same cursor, same offsets.
pub fn read(text: &str) -> Result<Document<Opaque>, KernelError> {
    read_as::<Opaque>(text, None)
}

/// The one reader. `expect` is the payload type the caller requires, or
/// `None` to accept whatever the file declares.
fn read_as<A: PayloadText>(text: &str, expect: Option<&str>) -> Result<Document<A>, KernelError> {
    // The version line is checked before trivia, because `%` also starts a
    // comment: a file without a header would otherwise parse as a file with
    // one missing, and a consumer would have no way to refuse a format it
    // does not know.
    let header = text.lines().next().unwrap_or_default().trim_end();
    if header != format!("% {FORMAT_VERSION}") {
        return Err(KernelError::Parse {
            offset: 0,
            message: format!("expected the header `% {FORMAT_VERSION}`, found `{header}`"),
        });
    }
    let mut cursor = Cursor::new(text);
    cursor.keyword("kernel")?;
    let name = cursor.string()?;
    cursor.symbol("{")?;
    cursor.keyword("composition")?;
    // The composition's name is not part of the meaning: a file has one
    // composition, and the term is what it denotes.
    drop(cursor.name()?);
    cursor.symbol(":")?;
    cursor.keyword("Timeline")?;
    cursor.symbol("[")?;
    let payload_type = cursor.name()?;
    if let Some(expected) = expect
        && payload_type != expected
    {
        return Err(cursor.error(format!("this file carries `{payload_type}` payloads, not `{expected}`")));
    }
    cursor.symbol("]")?;
    cursor.symbol("=")?;
    let term = cursor.term::<A>()?;
    cursor.symbol(";")?;
    cursor.symbol("}")?;
    cursor.end()?;
    Ok(Document {
        name,
        payload_type,
        notes: notes(text).map(str::to_owned).collect(),
        term,
    })
}

fn write_term<A: PayloadText>(out: &mut String, term: &Term<A>, depth: usize) {
    let pad = "  ".repeat(depth);
    let inner = "  ".repeat(depth + 1);
    match term.form() {
        Form::Literal(value) => {
            let _ = write!(out, "timeline {} {{", value.extent());
            for occurrence in value.occurrences() {
                let _ = write!(out, "\n{inner}occurrence ");
                write_string(out, &occurrence.payload().to_text());
                let _ = write!(
                    out,
                    " from {} to {};",
                    occurrence.span().start(),
                    occurrence.span().end()
                );
            }
            let _ = write!(out, "\n{pad}}}");
        }
        Form::Seq(parts) => write_block(out, "sequence", parts, depth),
        Form::Over(parts) => write_block(out, "overlay", parts, depth),
        Form::Shift { by, body } => {
            let _ = write!(out, "shift by {by} ");
            write_term(out, body, depth);
        }
        Form::Scale { by, body } => {
            let _ = write!(out, "scale by ");
            write_ratio(out, *by);
            out.push(' ');
            write_term(out, body, depth);
        }
        Form::Restrict { window, body } => {
            let _ = write!(out, "restrict from {} to {} ", window.start(), window.end());
            write_term(out, body, depth);
        }
        Form::Let { name, value, body } => {
            let _ = write!(out, "let {name} = ");
            write_term(out, value, depth + 1);
            let _ = write!(out, "\n{pad}in ");
            write_term(out, body, depth);
        }
        Form::Var { name, mark } => {
            out.push_str(name);
            if let Some(mark) = mark {
                out.push_str(" @ ");
                write_string(out, mark);
            }
        }
    }
}

fn write_block<A: PayloadText>(out: &mut String, keyword: &str, parts: &[Term<A>], depth: usize) {
    let pad = "  ".repeat(depth);
    let inner = "  ".repeat(depth + 1);
    let _ = write!(out, "{keyword} {{");
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            out.push(';');
        }
        let _ = write!(out, "\n{inner}");
        write_term(out, part, depth + 1);
    }
    let _ = write!(out, "\n{pad}}}");
}

fn write_ratio(out: &mut String, value: Ratio<i64>) {
    if *value.denom() == 1 {
        let _ = write!(out, "{}", value.numer());
    } else {
        let _ = write!(out, "{}/{}", value.numer(), value.denom());
    }
}

/// A double-quoted string with `"` and `\` escaped — the grammar's
/// `string-literal`. Payload text needs no lexical constraints of its own
/// because it is always carried this way.
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(character),
        }
    }
    out.push('"');
}

/// A hand-written recursive-descent cursor.
///
/// Deliberately not sharing `musa-language`'s lexer: kernel text and musa
/// source are two languages that change independently, and the only thing
/// they genuinely have in common — reading a rational — is four lines. Coupling
/// them would mean a surface-syntax change could break the interchange format.
struct Cursor<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, at: 0 }
    }

    fn error(&self, message: impl Into<String>) -> KernelError {
        KernelError::Parse {
            offset: self.at,
            message: message.into(),
        }
    }

    /// Skip whitespace and `%` line comments.
    fn trivia(&mut self) {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            self.at = self.text.len() - trimmed.len();
            if self.rest().starts_with('%') {
                let line = self
                    .rest()
                    .find('\n')
                    .map_or_else(|| self.rest().len(), |index| index + 1);
                self.at += line;
            } else {
                return;
            }
        }
    }

    fn rest(&self) -> &'a str {
        self.text.get(self.at..).unwrap_or_default()
    }

    fn symbol(&mut self, symbol: &str) -> Result<(), KernelError> {
        self.trivia();
        if self.rest().starts_with(symbol) {
            self.at += symbol.len();
            Ok(())
        } else {
            Err(self.error(format!("expected `{symbol}`")))
        }
    }

    fn peek_symbol(&mut self, symbol: &str) -> bool {
        self.trivia();
        self.rest().starts_with(symbol)
    }

    /// A bare word: a name or a keyword.
    fn word(&mut self) -> Result<&'a str, KernelError> {
        self.trivia();
        let rest = self.rest();
        let len = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
            .unwrap_or(rest.len());
        if len == 0 {
            return Err(self.error("expected a name"));
        }
        self.at += len;
        rest.get(..len).ok_or_else(|| self.error("expected a name"))
    }

    fn peek_word(&mut self) -> &'a str {
        let at = self.at;
        let word = self.word().unwrap_or_default();
        self.at = at;
        word
    }

    fn keyword(&mut self, keyword: &str) -> Result<(), KernelError> {
        let word = self.word()?;
        if word == keyword {
            Ok(())
        } else {
            Err(self.error(format!("expected `{keyword}`, found `{word}`")))
        }
    }

    fn name(&mut self) -> Result<String, KernelError> {
        Ok(self.word()?.to_owned())
    }

    fn string(&mut self) -> Result<String, KernelError> {
        self.symbol("\"")?;
        let mut value = String::new();
        let mut escaped = false;
        for character in self.rest().chars() {
            self.at += character.len_utf8();
            if escaped {
                value.push(match character {
                    'n' => '\n',
                    other => other,
                });
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                return Ok(value);
            } else {
                value.push(character);
            }
        }
        Err(self.error("unterminated string"))
    }

    fn ratio(&mut self) -> Result<Ratio<i64>, KernelError> {
        self.trivia();
        let rest = self.rest();
        let len = rest
            .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '/')
            .unwrap_or(rest.len());
        let text = rest.get(..len).unwrap_or_default();
        self.at += len;
        let (numer, denom) = text.split_once('/').unwrap_or((text, "1"));
        let numer: i64 = numer.parse().map_err(|_| self.error("expected a rational"))?;
        let denom: i64 = denom.parse().map_err(|_| self.error("expected a rational"))?;
        if denom == 0 {
            return Err(self.error("a rational's denominator is not zero"));
        }
        Ok(Ratio::new(numer, denom))
    }

    fn beat(&mut self) -> Result<Beat, KernelError> {
        Ok(Beat::new(self.ratio()?))
    }

    fn end(&mut self) -> Result<(), KernelError> {
        self.trivia();
        if self.rest().is_empty() {
            Ok(())
        } else {
            Err(self.error("unexpected trailing text"))
        }
    }

    fn term<A: PayloadText>(&mut self) -> Result<Term<A>, KernelError> {
        self.trivia();
        if self.peek_symbol("(") {
            self.symbol("(")?;
            let inner = self.term()?;
            self.symbol(")")?;
            return Ok(inner);
        }
        match self.peek_word() {
            "timeline" => self.literal(),
            "sequence" => {
                let parts = self.block()?;
                Term::seq(parts).map_err(|error| self.error(error.to_string()))
            }
            "overlay" => {
                let parts = self.block()?;
                Term::over(parts).map_err(|error| self.error(error.to_string()))
            }
            "scale" => {
                self.keyword("scale")?;
                self.keyword("by")?;
                let factor = self.ratio()?;
                let body = self.term()?;
                Term::scale(factor, body).map_err(|error| self.error(error.to_string()))
            }
            "shift" => {
                self.keyword("shift")?;
                self.keyword("by")?;
                let by = self.beat()?;
                let body = self.term()?;
                Term::shift(by, body).map_err(|error| self.error(error.to_string()))
            }
            "restrict" => {
                self.keyword("restrict")?;
                self.keyword("from")?;
                let start = self.beat()?;
                self.keyword("to")?;
                let end = self.beat()?;
                let window = Span::new(start, end)?;
                Ok(Term::restrict(window, self.term()?))
            }
            "let" => {
                self.keyword("let")?;
                let name = self.name()?;
                self.symbol("=")?;
                let value = self.term()?;
                self.keyword("in")?;
                let body = self.term()?;
                Ok(Term::bind(name, value, body))
            }
            "" => Err(self.error("expected a term")),
            _ => {
                let name = self.name()?;
                if self.peek_symbol("@") {
                    self.symbol("@")?;
                    return Ok(Term::var_marked(name, self.string()?));
                }
                Ok(Term::var(name))
            }
        }
    }

    fn block<A: PayloadText>(&mut self) -> Result<Vec<Term<A>>, KernelError> {
        let _ = self.word()?;
        self.symbol("{")?;
        let mut parts = Vec::new();
        loop {
            if self.peek_symbol("}") {
                self.symbol("}")?;
                return Ok(parts);
            }
            parts.push(self.term()?);
            if self.peek_symbol(";") {
                self.symbol(";")?;
            }
        }
    }

    fn literal<A: PayloadText>(&mut self) -> Result<Term<A>, KernelError> {
        self.keyword("timeline")?;
        let extent = self.beat()?;
        self.symbol("{")?;
        let mut occurrences = Vec::new();
        loop {
            if self.peek_symbol("}") {
                self.symbol("}")?;
                let value = timeline(extent, occurrences)?;
                return Ok(Term::literal(value));
            }
            self.keyword("occurrence")?;
            let text = self.string()?;
            let payload = A::from_text(&text).ok_or_else(|| self.error(format!("`{text}` is not a payload")))?;
            self.keyword("from")?;
            let start = self.beat()?;
            self.keyword("to")?;
            let end = self.beat()?;
            self.symbol(";")?;
            occurrences.push(Occurrence::new(Span::new(start, end)?, payload));
        }
    }
}
