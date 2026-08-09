//! Lowering for the Appendix-A, line-oriented RLM text surface.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use kittens_code_protocol::error::VerbErrorCause;

use super::ir::{
    Any, Binding, BoundValue, By, Chunks, EventKind, FinalValue, Instr, Out, Query, Range,
    RangeUnit, Records, Ref, Sel, VerbError,
};

/// Lowers a script with no caller-supplied verb-count limit.
///
/// Every nonempty physical line binds one one-based slot. Syntax and type
/// failures are represented as inline [`BoundValue::Error`] values, so a
/// later line can still lower. Empty and ASCII-whitespace-only lines do not
/// consume slots.
#[must_use]
pub fn lower_script(script: &str) -> Query {
    lower_script_with_verb_limit(script, usize::MAX)
}

/// Lowers a script while enforcing a query-level verb-count limit.
///
/// Otherwise-valid lines beyond `max_verbs` bind
/// [`VerbErrorCause::Budget`]. Syntax and type validation happens first so a
/// new budget check cannot mask an existing rejection oracle. This models the
/// Q5 verb-count meter and makes the protocol's budget cause observable at
/// the same inline binding boundary as all other verb failures.
#[must_use]
pub fn lower_script_with_verb_limit(script: &str, max_verbs: usize) -> Query {
    let mut query = Vec::new();
    let mut outputs = Vec::new();
    let mut slot = 0_u32;

    for physical_line in script.split('\n') {
        let line = physical_line.strip_suffix('\r').unwrap_or(physical_line);
        if line.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
            continue;
        }

        slot = slot.saturating_add(1);
        let value = match lower_line(line, &outputs) {
            Err(error) => BoundValue::Error(error),
            Ok(_) if query.len() >= max_verbs => BoundValue::Error(VerbError {
                verb: verb_hint(line),
                cause: VerbErrorCause::Budget,
            }),
            Ok(instruction) => BoundValue::Instr(instruction),
        };

        let output = match &value {
            BoundValue::Instr(instruction) => Some(instruction.output()),
            BoundValue::Error(_) => None,
        };
        outputs.push(output);
        query.push(Binding { slot, value });
    }

    query
}

fn verb_hint(line: &str) -> String {
    line.trim_start_matches([' ', '\t'])
        .split([' ', '\t'])
        .next()
        .unwrap_or("")
        .to_string()
}

fn lower_line(line: &str, outputs: &[Option<Out>]) -> Result<Instr, VerbError> {
    let (verb, args) = lex_line(line).map_err(|cause| VerbError {
        verb: verb_hint(line),
        cause,
    })?;

    let instruction = match verb.as_str() {
        "grep" => lower_grep(&args, outputs),
        "slice" => lower_slice(&args, outputs),
        "head" => lower_head_or_tail(&args, outputs, false),
        "tail" => lower_head_or_tail(&args, outputs, true),
        "count" => lower_count(&args, outputs),
        "partition" => lower_partition(&args, outputs),
        "ask" => lower_ask(&args, outputs),
        "ask-each" => lower_ask_each(&args, outputs),
        "final" => lower_final(&args, outputs),
        _ => Err(VerbErrorCause::Parse),
    };

    instruction.map_err(|cause| VerbError { verb, cause })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Atom {
    Ref(u32),
    Range(Range),
    Number(u64),
    String(String),
    Ident(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Flag {
    name: String,
    value: Option<Atom>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
    Atom(Atom),
    Flag(Flag),
}

#[derive(Debug)]
struct Args {
    positional: Vec<Atom>,
    flags: Vec<Flag>,
}

impl Args {
    fn from_tokens(tokens: Vec<Token>) -> Result<Self, VerbErrorCause> {
        let mut positional = Vec::new();
        let mut flags: Vec<Flag> = Vec::new();

        for token in tokens {
            match token {
                Token::Atom(atom) => positional.push(atom),
                Token::Flag(flag) => {
                    if flags.iter().any(|seen| seen.name == flag.name) {
                        return Err(VerbErrorCause::BadFlag);
                    }
                    flags.push(flag);
                }
            }
        }

        Ok(Self { positional, flags })
    }

    fn reject_unknown_flags(&self, allowed: &[&str]) -> Result<(), VerbErrorCause> {
        if self
            .flags
            .iter()
            .any(|flag| !allowed.contains(&flag.name.as_str()))
        {
            Err(VerbErrorCause::BadFlag)
        } else {
            Ok(())
        }
    }

    fn flag(&self, name: &str) -> Option<&Flag> {
        self.flags.iter().find(|flag| flag.name == name)
    }
}

fn lex_line(line: &str) -> Result<(String, Args), VerbErrorCause> {
    let mut lexer = Lexer::new(line);
    let first = lexer.next_token()?.ok_or(VerbErrorCause::Parse)?;
    let verb = match first {
        Token::Atom(Atom::Ident(verb)) => verb,
        Token::Atom(_) | Token::Flag(_) => return Err(VerbErrorCause::Parse),
    };

    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token()? {
        tokens.push(token);
    }
    let args = Args::from_tokens(tokens)?;
    Ok((verb, args))
}

struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    const fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn next_token(&mut self) -> Result<Option<Token>, VerbErrorCause> {
        self.skip_separator();
        if self.position == self.input.len() {
            return Ok(None);
        }

        if self.remaining().starts_with("--") {
            self.position += 2;
            return self.parse_flag().map(|flag| Some(Token::Flag(flag)));
        }

        self.parse_atom().map(|atom| Some(Token::Atom(atom)))
    }

    fn parse_flag(&mut self) -> Result<Flag, VerbErrorCause> {
        let start = self.position;
        let Some(first) = self.peek_char() else {
            return Err(VerbErrorCause::BadFlag);
        };
        if !first.is_ascii_alphabetic() {
            return Err(VerbErrorCause::BadFlag);
        }
        self.bump_char();
        while self
            .peek_char()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        {
            self.bump_char();
        }
        let name = self.input[start..self.position].to_string();

        let value = if self.peek_char() == Some('=') {
            self.bump_char();
            if self.position == self.input.len()
                || self.peek_char().is_some_and(is_ascii_separator)
                || self.remaining().starts_with("--")
            {
                return Err(VerbErrorCause::BadFlag);
            }
            Some(self.parse_atom().map_err(|_| VerbErrorCause::BadFlag)?)
        } else {
            None
        };

        self.require_boundary(VerbErrorCause::BadFlag)?;
        Ok(Flag { name, value })
    }

    fn parse_atom(&mut self) -> Result<Atom, VerbErrorCause> {
        let atom = match self.peek_char() {
            Some('"') => self.parse_string()?,
            Some('%') => self.parse_ref()?,
            Some(ch) if ch.is_ascii_digit() => self.parse_number()?,
            Some(ch) if ch.is_ascii_alphabetic() => self.parse_ident_or_range()?,
            Some(_) | None => return Err(VerbErrorCause::Parse),
        };
        self.require_boundary(match atom {
            Atom::Range(_) => VerbErrorCause::BadRange,
            _ => VerbErrorCause::Parse,
        })?;
        Ok(atom)
    }

    fn parse_string(&mut self) -> Result<Atom, VerbErrorCause> {
        self.bump_char();
        let mut value = String::new();

        loop {
            let Some(ch) = self.peek_char() else {
                return Err(VerbErrorCause::Parse);
            };
            self.bump_char();
            match ch {
                '"' => return Ok(Atom::String(value)),
                '\\' => {
                    let Some(escaped) = self.peek_char() else {
                        return Err(VerbErrorCause::Parse);
                    };
                    self.bump_char();
                    match escaped {
                        '"' | '\\' => value.push(escaped),
                        _ => return Err(VerbErrorCause::Parse),
                    }
                }
                '\r' | '\n' => return Err(VerbErrorCause::Parse),
                _ => value.push(ch),
            }
        }
    }

    fn parse_ref(&mut self) -> Result<Atom, VerbErrorCause> {
        self.bump_char();
        let start = self.position;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump_char();
        }
        if start == self.position {
            return Err(VerbErrorCause::Parse);
        }
        let line = self.input[start..self.position]
            .parse::<u32>()
            .map_err(|_| VerbErrorCause::BadRef)?;
        Ok(Atom::Ref(line))
    }

    fn parse_number(&mut self) -> Result<Atom, VerbErrorCause> {
        let start = self.position;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump_char();
        }
        let number = self.input[start..self.position]
            .parse::<u64>()
            .map_err(|_| VerbErrorCause::Parse)?;
        Ok(Atom::Number(number))
    }

    fn parse_ident_or_range(&mut self) -> Result<Atom, VerbErrorCause> {
        let start = self.position;
        self.bump_char();
        while self
            .peek_char()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        {
            self.bump_char();
        }
        let ident = &self.input[start..self.position];
        if self.peek_char() != Some(':') {
            return Ok(Atom::Ident(ident.to_string()));
        }

        self.bump_char();
        let unit = match ident {
            "turn" => RangeUnit::Turn,
            "seq" => RangeUnit::Seq,
            "byte" => RangeUnit::Byte,
            _ => return Err(VerbErrorCause::BadRange),
        };
        let start = self.parse_range_bound()?;
        if !self.remaining().starts_with("..") {
            return Err(VerbErrorCause::BadRange);
        }
        self.position += 2;
        let end = self.parse_range_bound()?;
        if start > end {
            return Err(VerbErrorCause::BadRange);
        }
        Ok(Atom::Range(Range { unit, start, end }))
    }

    fn parse_range_bound(&mut self) -> Result<u64, VerbErrorCause> {
        let start = self.position;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump_char();
        }
        if start == self.position {
            return Err(VerbErrorCause::BadRange);
        }
        self.input[start..self.position]
            .parse::<u64>()
            .map_err(|_| VerbErrorCause::BadRange)
    }

    fn require_boundary(&self, cause: VerbErrorCause) -> Result<(), VerbErrorCause> {
        if self.position == self.input.len() || self.peek_char().is_some_and(is_ascii_separator) {
            Ok(())
        } else {
            Err(cause)
        }
    }

    fn skip_separator(&mut self) {
        while self.peek_char().is_some_and(is_ascii_separator) {
            self.bump_char();
        }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.position..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn bump_char(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.position += ch.len_utf8();
        }
    }
}

const fn is_ascii_separator(ch: char) -> bool {
    matches!(ch, ' ' | '\t')
}

fn lower_grep(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&["ctx", "kind"])?;
    let ctx = match args.flag("ctx") {
        None => 0,
        Some(Flag {
            value: Some(Atom::Number(value)),
            ..
        }) => u16::try_from(*value).map_err(|_| VerbErrorCause::BadFlag)?,
        Some(_) => return Err(VerbErrorCause::BadFlag),
    };
    let kind = match args.flag("kind") {
        None => None,
        Some(Flag {
            value: Some(Atom::Ident(kind)),
            ..
        }) => Some(EventKind::new(kind.clone())),
        Some(_) => return Err(VerbErrorCause::BadFlag),
    };

    let (pattern, sel) = match args.positional.as_slice() {
        [Atom::String(pattern)] => (pattern.clone(), Sel::Whole),
        [Atom::String(pattern), selector] => (pattern.clone(), parse_sel(selector, outputs)?),
        _ => return Err(VerbErrorCause::Parse),
    };
    validate_pattern(&pattern)?;

    Ok(Instr::Grep {
        pattern,
        sel,
        ctx,
        kind,
    })
}

fn lower_slice(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&[])?;
    let sel = match args.positional.as_slice() {
        [] => Sel::Whole,
        [selector] => parse_sel(selector, outputs)?,
        _ => return Err(VerbErrorCause::Parse),
    };
    Ok(Instr::Slice { sel })
}

fn lower_head_or_tail(
    args: &Args,
    outputs: &[Option<Out>],
    tail: bool,
) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&[])?;
    let (n, sel) = match args.positional.as_slice() {
        [Atom::Number(n)] => (number_to_u32(*n)?, Sel::Whole),
        [Atom::Number(n), selector] => (number_to_u32(*n)?, parse_sel(selector, outputs)?),
        _ => return Err(VerbErrorCause::Parse),
    };
    if tail {
        Ok(Instr::Tail { sel, n })
    } else {
        Ok(Instr::Head { sel, n })
    }
}

fn lower_count(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&[])?;
    let (pattern, sel) = match args.positional.as_slice() {
        [] => (None, Sel::Whole),
        [Atom::String(pattern)] => (Some(pattern.clone()), Sel::Whole),
        [selector] => (None, parse_sel(selector, outputs)?),
        [Atom::String(pattern), selector] => (Some(pattern.clone()), parse_sel(selector, outputs)?),
        _ => return Err(VerbErrorCause::Parse),
    };
    if let Some(pattern) = &pattern {
        validate_pattern(pattern)?;
    }
    Ok(Instr::Count { pattern, sel })
}

fn lower_partition(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&["by", "size"])?;
    let by = match args.flag("by") {
        Some(Flag {
            value: Some(Atom::Ident(by)),
            ..
        }) => match by.as_str() {
            "turns" => By::Turns,
            "bytes" => By::Bytes,
            "regex" => By::Regex,
            _ => return Err(VerbErrorCause::BadFlag),
        },
        Some(_) | None => return Err(VerbErrorCause::BadFlag),
    };

    match by {
        By::Turns | By::Bytes => {
            let sel = match args.positional.as_slice() {
                [] => Sel::Whole,
                [selector] => parse_sel(selector, outputs)?,
                _ => return Err(VerbErrorCause::Parse),
            };
            let size = match args.flag("size") {
                Some(Flag {
                    value: Some(Atom::Number(size)),
                    ..
                }) => u32::try_from(*size).map_err(|_| VerbErrorCause::BadFlag)?,
                Some(_) | None => return Err(VerbErrorCause::BadFlag),
            };
            Ok(Instr::Partition {
                sel,
                by,
                size: Some(size),
                pattern: None,
            })
        }
        By::Regex => {
            if args.flag("size").is_some() {
                return Err(VerbErrorCause::BadFlag);
            }
            let (sel, pattern) = match args.positional.as_slice() {
                [Atom::String(pattern)] => (Sel::Whole, pattern.clone()),
                [selector, Atom::String(pattern)] => {
                    (parse_sel(selector, outputs)?, pattern.clone())
                }
                _ => return Err(VerbErrorCause::Parse),
            };
            validate_pattern(&pattern)?;
            Ok(Instr::Partition {
                sel,
                by,
                size: None,
                pattern: Some(pattern),
            })
        }
    }
}

fn lower_ask(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&["sample-k"])?;
    let sample_k = match args.flag("sample-k") {
        None => None,
        Some(Flag {
            value: Some(Atom::Number(sample_k)),
            ..
        }) => Some(u8::try_from(*sample_k).map_err(|_| VerbErrorCause::BadFlag)?),
        Some(_) => return Err(VerbErrorCause::BadFlag),
    };
    let (sel, question) = match args.positional.as_slice() {
        [Atom::String(question)] => (Sel::Whole, question.clone()),
        [selector, Atom::String(question)] => (parse_sel(selector, outputs)?, question.clone()),
        _ => return Err(VerbErrorCause::Parse),
    };
    Ok(Instr::Ask {
        sel,
        question,
        sample_k,
    })
}

fn lower_ask_each(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&[])?;
    let (chunks, question) = match args.positional.as_slice() {
        [Atom::Ref(line), Atom::String(question)] => {
            (parse_chunks_ref(*line, outputs)?, question.clone())
        }
        _ => return Err(VerbErrorCause::Parse),
    };
    Ok(Instr::AskEach { chunks, question })
}

fn lower_final(args: &Args, outputs: &[Option<Out>]) -> Result<Instr, VerbErrorCause> {
    args.reject_unknown_flags(&[])?;
    let value = match args.positional.as_slice() {
        [Atom::String(value)] => FinalValue::Literal(value.clone()),
        [Atom::Ref(line)] => FinalValue::Ref(parse_any_ref(*line, outputs)?),
        _ => return Err(VerbErrorCause::Parse),
    };
    Ok(Instr::Final { value })
}

fn parse_sel(atom: &Atom, outputs: &[Option<Out>]) -> Result<Sel, VerbErrorCause> {
    match atom {
        Atom::Ref(line) => Ok(Sel::Ref(parse_records_ref(*line, outputs)?)),
        Atom::Range(range) => Ok(Sel::Range(*range)),
        Atom::Number(_) | Atom::String(_) | Atom::Ident(_) => Err(VerbErrorCause::Parse),
    }
}

fn parse_records_ref(line: u32, outputs: &[Option<Out>]) -> Result<Ref<Records>, VerbErrorCause> {
    if referenced_output(line, outputs)? == Out::Records {
        Ok(Ref::new(line))
    } else {
        Err(VerbErrorCause::BadRef)
    }
}

fn parse_chunks_ref(line: u32, outputs: &[Option<Out>]) -> Result<Ref<Chunks>, VerbErrorCause> {
    if referenced_output(line, outputs)? == Out::Chunks {
        Ok(Ref::new(line))
    } else {
        Err(VerbErrorCause::BadRef)
    }
}

fn parse_any_ref(line: u32, outputs: &[Option<Out>]) -> Result<Ref<Any>, VerbErrorCause> {
    referenced_output(line, outputs)?;
    Ok(Ref::new(line))
}

fn referenced_output(line: u32, outputs: &[Option<Out>]) -> Result<Out, VerbErrorCause> {
    let index = line.checked_sub(1).ok_or(VerbErrorCause::BadRef)?;
    let index = usize::try_from(index).map_err(|_| VerbErrorCause::BadRef)?;
    outputs
        .get(index)
        .copied()
        .flatten()
        .ok_or(VerbErrorCause::BadRef)
}

fn number_to_u32(number: u64) -> Result<u32, VerbErrorCause> {
    u32::try_from(number).map_err(|_| VerbErrorCause::Parse)
}

fn validate_pattern(pattern: &str) -> Result<(), VerbErrorCause> {
    let bytes = pattern.as_bytes();
    for index in 0..bytes.len().saturating_sub(2) {
        if bytes[index] != b'(' || bytes[index + 1] != b'?' {
            continue;
        }
        let mut preceding_backslashes = 0;
        let mut cursor = index;
        while cursor > 0 && bytes[cursor - 1] == b'\\' {
            preceding_backslashes += 1;
            cursor -= 1;
        }
        if preceding_backslashes % 2 == 0
            && matches!(
                bytes[index + 2],
                b'i' | b'm' | b's' | b'R' | b'U' | b'u' | b'x' | b'-'
            )
        {
            return Err(VerbErrorCause::Parse);
        }
    }
    Ok(())
}
