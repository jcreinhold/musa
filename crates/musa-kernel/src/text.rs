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

/// A payload that can cross the interchange boundary.
///
/// The two directions are one trait because they are one decision: a payload
/// text form that cannot be read back is not a text form, and N3 already
/// requires the writer to be injective, which is exactly the round-trip
/// property. Implementations live with the payload — `ScoreFact`'s is in
/// `musa-compiler` — and the kernel never looks inside the string.
pub trait TextPayload: Sized {
    /// The payload's text form. Must be injective on values (N3).
    fn to_text(&self) -> String;

    /// The inverse of [`Self::to_text`], or `None` if `text` is not one.
    fn from_text(text: &str) -> Option<Self>;

    /// The payload type's name, as the file's `Timeline[…]` annotation.
    fn type_name() -> &'static str;
}

/// The format version written into every file's header.
///
/// A string, not a negotiation scheme: a reader that does not recognise it
/// should say so and stop, which is all a version needs to do here.
pub const FORMAT_VERSION: &str = "musa-kernel-1";

/// Print `term` as a kernel file named `name`.
///
/// The term is printed as written: `let`, `seq`, `over`, `shift` and `scale`
/// structure survives. Normalizing first is the caller's separate decision —
/// there is no "normalize while printing" flag, because that would complect
/// two choices a caller can make in sequence.
pub fn print<A: TextPayload>(name: &str, term: &Term<A>) -> String {
    let mut out = String::with_capacity(256);
    let _ = writeln!(out, "% {FORMAT_VERSION}");
    let _ = write!(out, "kernel ");
    write_string(&mut out, name);
    let _ = writeln!(out, " {{");
    let _ = write!(out, "  composition main : Timeline[{}] =\n    ", A::type_name());
    write_term(&mut out, term, 2);
    let _ = writeln!(out, ";");
    let _ = writeln!(out, "}}");
    out
}

/// Parse a kernel file, returning its name and its term.
///
/// The term is *not* checked: [`Term::check`] is the caller's next call, and
/// keeping the two apart means a parse error and a well-formedness error stay
/// distinguishable to whoever reports them.
///
/// # Errors
///
/// [`KernelError::Parse`], naming the byte offset and what was expected.
pub fn parse<A: TextPayload>(text: &str) -> Result<(String, Term<A>), KernelError> {
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
    if payload_type != A::type_name() {
        return Err(cursor.error(format!(
            "this file carries `{payload_type}` payloads, not `{}`",
            A::type_name()
        )));
    }
    cursor.symbol("]")?;
    cursor.symbol("=")?;
    let term = cursor.term::<A>()?;
    cursor.symbol(";")?;
    cursor.symbol("}")?;
    cursor.end()?;
    Ok((name, term))
}

fn write_term<A: TextPayload>(out: &mut String, term: &Term<A>, depth: usize) {
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

fn write_block<A: TextPayload>(out: &mut String, keyword: &str, parts: &[Term<A>], depth: usize) {
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

    fn term<A: TextPayload>(&mut self) -> Result<Term<A>, KernelError> {
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

    fn block<A: TextPayload>(&mut self) -> Result<Vec<Term<A>>, KernelError> {
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

    fn literal<A: TextPayload>(&mut self) -> Result<Term<A>, KernelError> {
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
