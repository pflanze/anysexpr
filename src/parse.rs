// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::pos::Pos;
use crate::value::{Atom, Parenkind};
use crate::number::R5RSNumber;
use num::{BigInt, rational::Ratio};
use kstring::KString;
use thiserror::Error;
use genawaiter::rc::Gen;
use std::char::CharTryFromError;
use std::fmt::Write;
use std::convert::TryFrom;

fn take_while_and_rest<'s>(
    s: &'s str, pred: impl Fn(char) -> bool
) -> (&'s str, &'s str) {
    if let Some(i) = s.find(|c| ! pred(c)) {
        (&s[0..i], &s[i..])
    } else {
        (&s, "")
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error ({0}) after")]
    // XX: should not use anyhow::Error in buffered_chars.rs
    IOError(anyhow::Error),
    #[error("unexpected EOF in string/symbol delimited by '{0}' starting")]
    UnexpectedEOFInString(char),
    #[error("too many semicolons to start a comment")]
    TooManySemicolons,
    #[error("invalid escaped character '{0}'")]
    InvalidEscapedChar(char),
    #[error("not a hex digit: '{0}'")]
    NonHexDigit(char),
    #[error("invalid code point {0}")]
    InvalidCodePoint(CharTryFromError),
    #[error("missing delimiter '{0}' after code sequence")]
    MissingDelimiterForCodeSequence(char),
    #[error("too many digits in code sequence")]
    TooManyDigits,
    #[error("invalid '#' token")]
    InvalidHashToken,
}

#[derive(Error, Debug)]
#[error("{err} {pos}")]
pub struct ParseErrorWithPos {
    pub err: ParseError,
    pub pos: Pos
}

impl ParseError {
    fn at(self, p: Pos) -> ParseErrorWithPos {
        ParseErrorWithPos {
            err: self,
            pos: p
        }
    }
}

pub fn maybe_open_close(c: char) -> Option<Token> {
    match c {
        '(' => Some(Token::Open(Parenkind::Round)),
        '[' => Some(Token::Open(Parenkind::Square)),
        '{' => Some(Token::Open(Parenkind::Curly)),
        ')' => Some(Token::Close(Parenkind::Round)),
        ']' => Some(Token::Close(Parenkind::Square)),
        '}' => Some(Token::Close(Parenkind::Curly)),
        _ => None
    }
}

#[derive(Debug)]
pub enum CommentStyle {
    Singleline(u8), // ;  ;;  ;;;  etc.
    // XXX todo: multiline, sexpr-comments
}

#[derive(Debug)]
pub enum Token {
    Atom(Atom),
    Dot,
    Quasiquote,
    Quote,
    Unquote,
    Open(Parenkind),
    Close(Parenkind),
    Whitespace(KString),
    Comment(CommentStyle, KString),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            Token::Atom(a) => a.fmt(f),
            Token::Dot => f.write_char('.'),
            Token::Quasiquote => f.write_char('`'),
            Token::Quote => f.write_char(','),
            Token::Unquote => f.write_char(','),
            Token::Open(k) => f.write_char(k.opening()),
            Token::Close(k) => f.write_char(k.closing()),
            Token::Whitespace(s) => f.write_str(s),
            Token::Comment(style, s) => {
                match style {
                    CommentStyle::Singleline(n) => {
                        for _ in 0..*n {
                            f.write_char(';')?
                        }
                        f.write_str(s)
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TokenWithPos(pub Token, pub Pos);


fn read_number(is_neg: bool, s: &str) -> Option<R5RSNumber> {
    let mut n: BigInt = 0.into();
    let mut cs = s.chars();
    while let Some(c) = cs.next() {
        if c.is_ascii_digit() {
            n = n * 10 + c.to_digit(10).unwrap();
        } else if c == '/' {
            let numer = n;
            let mut n: BigInt = 0.into();
            while let Some(c) = cs.next() {
                if c.is_ascii_digit() {
                    n = n * 10 + c.to_digit(10).unwrap();
                } else {
                    return None;
                }
            }
            let denom = n;
            let n = Ratio::<BigInt>::new(numer, denom);
            return Some(R5RSNumber::Rational(Box::new(if is_neg { -n } else { n })))
        } else {
            // XXX: floating point, complex, and all the mixes.
            return None
        }
    }
    Some(R5RSNumber::Integer(if is_neg { -n } else { n }))
}

fn delimiter2maybe_stringlike_constructor(c: char) -> Option<fn(KString) -> Atom> {
    match c {
        '"' => Some(Atom::String),
        '|' => Some(Atom::Symbol),
        _  => None
    }
}

// c is a unicode code point
fn parse_hexdigit(c: u32) -> Option<u32> {
    if '0' as u32 <= c && c <= '9' as u32 {
        Some(c - '0' as u32)
    } else if 'a' as u32 <= c && c <= 'f' as u32 {
        Some(c - 'a' as u32 + 10)
    } else if 'A' as u32 <= c && c <= 'F' as u32 {
        Some(c - 'F' as u32 + 10)
    } else {
        None
    }
}

// s must be a hex string to the end or None is returned.
fn parse_as_hexstr(s: &str) -> Option<u32> {
    if s.len() > 8 {
        return None
    }
    let mut n = 0;
    for b in s.bytes() {
        n = n * 16 + parse_hexdigit(b as u32)?;
    }
    Some(n)
}

// Reads exactly numdigits digits, or up to the given delimiter, in
// which case numdigits is the max digits allowed
fn read_hex(
    outerdelimiter: char,
    cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>,
    codestartpos: Pos,
    delimiter: Option<char>,
    numdigits: u32,
) -> Result<u32, ParseErrorWithPos> {
    let mut res: u32 = 0;
    let mut lastpos = codestartpos;
    let mut numdigits_seen = 0;
    // Checking numdigits_seen in two places: if delimiter given, we
    // need to read cs.next even if we have all characters already.
    while delimiter.is_some() || numdigits_seen < numdigits {
        if let Some(r) = cs.next() {
            match r {
                Err(e) => return Err(ParseError::IOError(e).at(lastpos)),
                Ok((c, pos)) => {
                    if let Some(delim) = delimiter {
                        if c == delim {
                            return Ok(res)
                        }
                    }
                    if let Some(n) = parse_hexdigit(c as u32) {
                        // This check is superfluous if no delimiter
                        // was given, but the alternatives appear more
                        // complicated.
                        if numdigits_seen == numdigits {
                            return Err(ParseError::TooManyDigits.at(pos))
                        }
                        res *= 16;
                        res += n;
                        numdigits_seen += 1;
                        lastpos = pos;
                    } else {
                        return Err(ParseError::NonHexDigit(c).at(pos))
                    }
                }
            }
        } else {
            return Err(ParseError::UnexpectedEOFInString(outerdelimiter).at(lastpos));
        }
    }
    if let Some(delim) = delimiter {
        Err(ParseError::MissingDelimiterForCodeSequence(delim).at(codestartpos))
    } else {
        Ok(res)
    }
}

fn read_hex_char(
    outerdelimiter: char,
    cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>,
    lastpos: Pos,
    delimiter: Option<char>,
    numdigits: u32,
) -> Result<char, ParseErrorWithPos> {
    let code = read_hex(outerdelimiter, cs, lastpos, delimiter, numdigits)?;
    match code.try_into() {
        Err(e) => Err(ParseError::InvalidCodePoint(e).at(lastpos)),
        Ok(c) => Ok(c)
    }
}

fn read_delimited(startpos: Pos,
                  cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>,
                  delimiter: char,
                  out: &mut String)
                  -> Result<(), ParseErrorWithPos> {
    out.clear();
    let mut escaped = false;
    let mut lastpos = startpos;
    let mut maybe_next_c_pos = None;
    loop {
        let c;
        let pos;
        if let Some(cp) = maybe_next_c_pos {
            (c, pos) = cp;
            maybe_next_c_pos = None;
        } else {
            if let Some(r) = cs.next() {
                match r {
                    Err(e) => return Err(ParseError::IOError(e).at(lastpos)),
                    Ok(cp) => {
                        (c, pos) = cp;
                    }
                }
            } else {
                return Err(ParseError::UnexpectedEOFInString(delimiter).at(startpos));
            }
        }
        lastpos = pos;
        if escaped {
            // https://small.r7rs.org/attachment/r7rs.pdf 6.7. Strings
            let replacement = match c {
                'a' => "\x07", // alarm
                'b' => "\x08", // backspace
                't' => "\t",
                'n' => "\n",
                'r' => "\r",
                // (Not in R7RS(?), but why not?: man ascii
                'v' => "\x0B",
                'f' => "\x0C",
                // Supported by Guile (Gambit reads more digits):
                '0' => "\0",
                // /Not in R7RS)
                '\\' => "\\",
                '"' => "\"", // possible delimiter
                '\'' => "\'",
                '|' => "|", // possible delimiter
                'u' => {
                    out.push(
                        read_hex_char(delimiter, cs, pos, None, 4)?);
                    ""
                }
                'U' => {
                    // Supported by Gambit, not Guile
                    out.push(
                        read_hex_char(delimiter, cs, pos, None, 8)?);
                    ""
                }
                'x' => {
                    // R7RS: Always terminated by a ';'
                    out.push(
                        read_hex_char(delimiter, cs, pos, Some(';'), 8)?);
                    // XX Guile: always read exactly 2 digits:
                    // out.push(
                    //     read_hex_char(delimiter, cs, pos, None, 2)?);
                    // XX Gambit: reads as many digits as there are (huh)
                    // both do not include the ';' in the sequence.
                    ""
                }
                '\n' => {
                    // Line continuation
                    let (_lastc, mcp)=
                        read_while(Some(c), pos, cs,
                                   is_whitespace_char, None)?;
                    if mcp.is_none() {
                        return Err(ParseError::UnexpectedEOFInString(
                            delimiter).at(startpos))
                    }
                    maybe_next_c_pos = mcp;
                    ""
                }
                _ => {
                    if c.is_ascii_digit() {
                        // Not in R7RS(?), but supported
                        // by Gambit Scheme, but not by
                        // Guile. Ignore?

                        // How do these work?
                        // > (map char->integer (string->list "\322"))
                        // (210)
                        // > (map char->integer (string->list "\422"))
                        // (34 50)
                        // > (map char->integer (string->list "\0"))    
                        // (0)
                        // > (map char->integer (string->list "\00"))
                        // (0)
                        // > (map char->integer (string->list "\010"))
                        // (8)
                        // > (map char->integer (string->list "\10")) 
                        // (8)
                        // Gambit reads up to 3 digits, it
                        // seems, or rather until before the
                        // number exceepds 255?
                        todo!()
                    } else {
                        return Err(ParseError::InvalidEscapedChar(c).at(pos))
                    }
                }
            };
            out.push_str(replacement);
            escaped = false;
        } else {
            if c == '\\' {
                escaped = true;
            } else if c == delimiter {
                return Ok(());
            } else {
                out.push(c);
            }
        }
    }
}

// Returns (, None) iff reached EOF;
// returns (None, ) iff reached EOF at the begin and no c was given.
fn read_while(
    c: Option<char>,
    startpos: Pos,
    cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>,
    accepted: fn(char) -> bool,
    mut opt_out: Option<&mut String>,
) -> Result<(Option<char>, Option<(char, Pos)>),
            ParseErrorWithPos> {
    if let Some(ref mut out) = opt_out {
        out.clear();
        if let Some(c) = c {
            out.push(c);
        }
    }
    let mut lastc = c;
    let mut lastpos = startpos;
    loop {
        if let Some(r) = cs.next() {
            match r {
                Err(e) => return Err(ParseError::IOError(e).at(lastpos)),
                Ok((c, pos)) => {
                    lastpos = pos;
                    if accepted(c) {
                        if let Some(ref mut out) = opt_out {
                            out.push(c);
                        }
                        lastc = Some(c);
                    } else {
                        return Ok((lastc, Some((c, pos))));
                    }
                }
            }
        } else {
            return Ok((lastc, None))
        }
    }
}

fn char2special_token(c: char) -> Option<Token> {
    match c {
        '\'' => Some(Token::Quote),
        '`' => Some(Token::Quasiquote),
        ',' => Some(Token::Unquote),
        // Not adding Dot here because '.' is also allowed as the
        // start of symbols.
        _ => None
    }
}    

fn is_symbol_or_number_char(c: char) -> bool {
    c.is_whitespace() == false
        && char2special_token(c).is_none()
        && delimiter2maybe_stringlike_constructor(c).is_none()
        && maybe_open_close(c).is_none()
        && c != '\\'
}

fn is_whitespace_char(c: char) -> bool {
    c.is_whitespace()
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

#[derive(Debug)]
pub struct ParseSettings {
    pub whitespace: bool,
    pub comments: bool,
}

pub fn parse(
    cs: impl Iterator<Item = anyhow::Result<(char, Pos)>>,
    settings: ParseSettings,
)
    -> impl Iterator<Item = Result<TokenWithPos, ParseErrorWithPos>>
{
    Gen::new(|co| async move {
        let mut cs = cs;
        let mut tmp = String::new();
        let mut maybe_next_c_pos = None;
        let mut lastpos = Pos { line: 0, col: 0 };
        loop {
            let c;
            let pos;
            if let Some(cp) = maybe_next_c_pos {
                (c, pos) = cp;
                maybe_next_c_pos = None;
            } else {
                if let Some(r) = cs.next() {
                    match r {
                        Err(e) => {
                            co.yield_(Err(
                                ParseError::IOError(e).at(lastpos))).await;
                            return;
                        }
                        Ok(cp) => {
                            (c, pos) = cp;
                        }
                    }
                } else {
                    return;
                }
            }
            lastpos = pos;
            
            if let Some(t) = maybe_open_close(c) {
                co.yield_(Ok(TokenWithPos(t, pos))).await;
            } else if c.is_whitespace() {
                if settings.whitespace {
                    match read_while(Some(c), pos, &mut cs, is_whitespace_char,
                                     Some(&mut tmp)) {
                        Err(e) => {
                            co.yield_(Err(e)).await;
                            return;
                        }
                        Ok((_lastc, mcp)) => {
                            co.yield_(
                                Ok(
                                    TokenWithPos(
                                        Token::Whitespace(KString::from_ref(&tmp)),
                                        pos))).await;
                            if mcp.is_none() {
                                // avoid calling next() again!
                                return
                            }
                            maybe_next_c_pos = mcp;
                        }
                    }
                }
            } else if c == ';' {
                // line comments
                match read_while(Some(c), pos, &mut cs, |c| c != '\n',
                                 Some(&mut tmp)) {
                    Err(e) => {
                        co.yield_(Err(e)).await;
                        return;
                    }
                    Ok((_lastc, mcp)) => {
                        if settings.comments {
                            let (start, rest) =
                                take_while_and_rest(&tmp, |c| c == ';');
                            let nsemicolons = start.len();
                            if let Ok(nsemi) = u8::try_from(nsemicolons) {
                                co.yield_(
                                    Ok(
                                        TokenWithPos(
                                            Token::Comment(
                                                CommentStyle::Singleline(nsemi),
                                                KString::from_ref(rest)),
                                            pos))).await;
                            } else {
                                co.yield_(Err(ParseError::TooManySemicolons.at(pos)))
                                    .await
                            }
                        }
                        if mcp.is_none() {
                            // avoid calling next() again!
                            return
                        }
                        maybe_next_c_pos = mcp;
                    }
                }
            } else if c == '#' {
                // #f #t #true #false #\character #:keyword #!special #<structure >
                let c0;
                if let Some(r) = cs.next() {
                    match r {
                        Err(e) => {
                            co.yield_(Err(
                                ParseError::IOError(e).at(lastpos))).await;
                            return;
                        }
                        Ok(cp) => {
                            c0 = cp.0;
                            lastpos = cp.1;
                        }
                    }
                } else {
                    co.yield_(Err(ParseError::InvalidHashToken.at(pos))).await;
                    return;
                }

                if c0 == '\\' {
                    // #\character
                    match read_while(None, pos, &mut cs, is_symbol_or_number_char,
                                     Some(&mut tmp)) {
                        Err(e) => {
                            co.yield_(Err(e)).await;
                            return;
                        }
                        Ok((_lastc, mcp)) => {
                            maybe_next_c_pos = mcp;
                            let r = (|| {
                                let len = tmp.len();
                                if len == 0 {
                                    return Err(ParseError::InvalidHashToken.at(pos))
                                }
                                let c0 = tmp.chars().next().unwrap();
                                if len == 1 {
                                    return Ok(c0)
                                }
                                if c0 == 'x' || c0 == 'u' || c0 == 'U' {
                                    // XX should we refuse lengths
                                    // other than 4 for u and 8 for U?
                                    // What about x?
                                    return
                                        if let Some(n) = parse_as_hexstr(&tmp[1..]) {
                                            match n.try_into() {
                                                Err(e) =>
                                                    Err(
                                                        ParseError::InvalidCodePoint(e)
                                                            .at(pos)),
                                                Ok(c) => Ok(c)
                                            }
                                        } else {
                                            Err(ParseError::InvalidHashToken.at(pos))
                                        };
                                }
                                if let Some(c) = crate::value::name2char(&tmp) {
                                    return Ok(c)
                                }
                                Err(ParseError::InvalidHashToken.at(pos))
                            })();
                            match r {
                                Err(e) => {
                                    co.yield_(Err(e)).await;
                                    return;
                                }
                                Ok(c) => co.yield_(Ok(TokenWithPos(
                                    Token::Atom(Atom::Char(c)),
                                    pos))).await
                            }
                        }
                    }

                } else {
                    // #true #false #:keyword #!special #<structure >
                    
                    match read_while(Some(c0), pos, &mut cs, |c| c.is_ascii_alphabetic(),
                                     Some(&mut tmp)) {
                        Err(e) => {
                            co.yield_(Err(e)).await;
                            return;
                        }
                        Ok((_lastc, mcp)) => {
                            maybe_next_c_pos = mcp;
                            let r = (|| {
                                let len = tmp.len();
                                if len == 0 {
                                    return Err(ParseError::InvalidHashToken.at(pos))
                                }
                                if len == 1 {
                                    match c0 {
                                        'f' => return Ok(Atom::Bool(false)),
                                        't' => return Ok(Atom::Bool(true)),
                                        _ => {}
                                    }
                                }

                                // XXX others
                                Err(ParseError::InvalidHashToken.at(pos))
                            })();
                            match r {
                                Err(e) => {
                                    co.yield_(Err(e)).await;
                                    return;
                                }
                                Ok(v) => co.yield_(Ok(TokenWithPos(
                                    Token::Atom(v),
                                    pos))).await
                            }
                        }
                    }
                }
            } else if let Some(constructor) =
                delimiter2maybe_stringlike_constructor(c)
            {
                match read_delimited(pos, &mut cs, c, &mut tmp) {
                    Err(e) => {
                        co.yield_(Err(e)).await;
                        return;
                    }
                    Ok(()) => {
                        co.yield_(Ok(
                            TokenWithPos(
                                Token::Atom(
                                    constructor(KString::from_ref(&tmp))),
                                pos))).await;
                    }
                }
            } else if let Some(t) = char2special_token(c) {
                co.yield_(Ok(TokenWithPos(t, pos))).await;
            } else {
                // Numbers, symbols, keywords, Dot
                match read_while(Some(c), pos, &mut cs, is_symbol_or_number_char,
                                 Some(&mut tmp)) {
                    Err(e) => {
                        co.yield_(Err(e)).await;
                        return;
                    }
                    Ok((lastc, mcp)) => {
                        let lastc = lastc.unwrap();
                        let r = (|| {
                            if tmp.len() == 1 && lastc == '.' {
                                return Ok(TokenWithPos(Token::Dot, pos));
                            }
                            if is_digit(c) {
                                if let Some(r) = read_number(false, &tmp) {
                                    return Ok(TokenWithPos(
                                        Token::Atom(Atom::Number(r)),
                                        pos))
                                }
                            } else if c == '-' {
                                if let Some(r) = read_number(true, &tmp[1..]) {
                                    return Ok(TokenWithPos(
                                        Token::Atom(Atom::Number(r)),
                                        pos))
                                }
                            }
                            let (constructor, s)
                                : (fn(KString) -> Atom, &str) =
                                if c == ':' {
                                    (Atom::Keyword1, &tmp[1..])
                                } else if lastc == ':' {
                                    (Atom::Keyword2, &tmp[0..tmp.len()-1])
                                } else {
                                    (Atom::Symbol, &tmp[0..])
                                };
                            Ok(
                                TokenWithPos(
                                    Token::Atom(
                                        constructor(KString::from_ref(s))),
                                    pos))
                        })();
                        co.yield_(r).await;
                        if mcp.is_none() {
                            // avoid calling next() again!
                            return
                        }
                        maybe_next_c_pos = mcp;
                    }
                }
            }
        }
    }).into_iter()
}
