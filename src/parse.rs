use crate::pos::Pos;
use crate::value::{Atom, Parenkind};
use crate::number::R5RSNumber;
use num::BigInt;
use kstring::KString;
use thiserror::Error;
use genawaiter::rc::Gen;
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
    #[error("IO error after {0}: {1}")]
    // XX: should not use anyhow::Error in buffered_chars.rs
    IOError(Pos, anyhow::Error),
    #[error("unexpected EOF in string/symbol delimited by '{1}' starting {0}")]
    UnexpectedEOF(Pos, char),
    #[error("too many semicolons to start a comment {0}")]
    TooManySemicolons(Pos),
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


fn read_number(c: char,
               startpos: Pos,
               cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>)
               -> Result<(Option<(char, Pos)>, Token), ParseError> {
    let mut n = BigInt::from(c.to_digit(10).unwrap());
    let mut lastpos = startpos;
    loop {
        if let Some(r) = cs.next() {
            match r {
                Err(e) => return Err(ParseError::IOError(lastpos, e)),
                Ok((c, pos)) => {
                    if c.is_ascii_digit() {
                        n = n * 10 + c.to_digit(10).unwrap();
                        lastpos = pos;
                    } else {
                        return Ok((Some((c, pos)),
                                   Token::Atom(Atom::Number(R5RSNumber::Integer(n)))));
                    }
                }
            }
        } else {
            return Ok((None,
                       Token::Atom(Atom::Number(R5RSNumber::Integer(n)))));
        }
    }
}

fn delimiter2maybe_stringlike_constructor(c: char) -> Option<fn(KString) -> Atom> {
    match c {
        '"' => Some(Atom::String),
        '|' => Some(Atom::Symbol),
        _  => None
    }
}

fn read_delimited(startpos: Pos,
                  cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>,
                  delimiter: char,
                  out: &mut String)
                  -> Result<(), ParseError> {
    out.clear();
    let mut escaped = false;
    let mut lastpos = startpos;
    loop {
        if let Some(r) = cs.next() {
            match r {
                Err(e) => return Err(ParseError::IOError(lastpos, e)),
                Ok((c, pos)) => {
                    lastpos = pos;
                    if escaped {
                        // XXX \n etc., and unicode escapes
                        out.push(c);
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
        } else {
            return Err(ParseError::UnexpectedEOF(startpos, delimiter));
        }
    }
}

// Returns (char, None) iff reached EOF
fn read_while(c: char,
              startpos: Pos,
              cs: &mut impl Iterator<Item = anyhow::Result<(char, Pos)>>,
              accepted: fn(char) -> bool,
              out: &mut String)
              -> Result<(char, Option<(char, Pos)>), ParseError> {
    out.clear();
    out.push(c);
    let mut lastc = c;
    let mut lastpos = startpos;
    loop {
        if let Some(r) = cs.next() {
            match r {
                Err(e) => return Err(ParseError::IOError(lastpos, e)),
                Ok((c, pos)) => {
                    lastpos = pos;
                    if accepted(c) {
                        out.push(c);
                        lastc = c;
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

fn is_symbol_char(c: char) -> bool {
    c.is_whitespace() == false
        && char2special_token(c).is_none()
        && delimiter2maybe_stringlike_constructor(c).is_none()
        && maybe_open_close(c).is_none()
        && c != '\\'
}

fn is_whitespace_char(c: char) -> bool {
    c.is_whitespace()
}

#[derive(Debug)]
pub struct ParseSettings {
    pub furnish_whitespace: bool,
    pub furnish_comments: bool,
}

pub fn parse(
    cs: impl Iterator<Item = anyhow::Result<(char, Pos)>>,
    settings: ParseSettings,
)
    -> impl Iterator<Item = Result<TokenWithPos, ParseError>>
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
                                ParseError::IOError(
                                    lastpos, e))).await;
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
                if settings.furnish_whitespace {
                    tmp.clear();
                    match read_while(c, pos, &mut cs, is_whitespace_char, &mut tmp) {
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
                tmp.clear();
                match read_while(c, pos, &mut cs, |c| c != '\n', &mut tmp) {
                    Err(e) => {
                        co.yield_(Err(e)).await;
                        return;
                    }
                    Ok((_lastc, mcp)) => {
                        if settings.furnish_comments {
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
                                co.yield_(Err(ParseError::TooManySemicolons(pos))).await
                            }
                        }
                        if mcp.is_none() {
                            // avoid calling next() again!
                            return
                        }
                        maybe_next_c_pos = mcp;
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
                if c.is_ascii_digit() {
                    // XXX todo: fall back to symbol/keyword parsing for non-numbers
                    match read_number(c, pos, &mut cs) {
                        Ok((mcp, t)) => {
                            co.yield_(Ok(TokenWithPos(t, pos))).await;
                            maybe_next_c_pos = mcp;
                        }
                        Err(e) => {
                            co.yield_(Err(e)).await;
                            return;
                        }
                    }
                } else {
                    // Symbols, keywords, Dot
                    tmp.clear();
                    match read_while(c, pos, &mut cs, is_symbol_char, &mut tmp) {
                        Err(e) => {
                            co.yield_(Err(e)).await;
                            return;
                        }
                        Ok((lastc, mcp)) => {
                            if tmp.len() == 1 && lastc == '.' {
                                co.yield_(Ok(TokenWithPos(Token::Dot, pos))).await;
                            } else {
                                let (constructor, s)
                                    : (fn(KString) -> Atom, &str) =
                                    if c == ':' {
                                        (Atom::Keyword1, &tmp[1..])
                                    } else if lastc == ':' {
                                        (Atom::Keyword2, &tmp[0..tmp.len()-1])
                                    } else {
                                        (Atom::Symbol, &tmp[0..])
                                    };
                                co.yield_(
                                    Ok(
                                        TokenWithPos(
                                            Token::Atom(
                                                constructor(KString::from_ref(s))),
                                            pos))).await;
                            }
                            if mcp.is_none() {
                                // avoid calling next() again!
                                return
                            }
                            maybe_next_c_pos = mcp;
                        }
                    }
                }
            }
        }
    }).into_iter()
}
