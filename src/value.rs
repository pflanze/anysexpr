// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Runtime data types representing an S-expression value.

//! Whereas [Atom](Atom) does not include lists, [VValue](VValue) adds
//! lists implemented using Rust vectors. [VValue](VValue) can
//! represent improper lists only as long as there are no cycles.

use crate::number::R5RSNumber;
use std::fmt::Write;
use kstring::KString;

#[derive(Debug)]
pub enum Atom {
    Bool(bool),
    Char(char),
    String(KString),
    Symbol(KString),
    Keyword1(KString), // :foo
    Keyword2(KString), // foo:
    Number(R5RSNumber),
}

fn fmt_stringlike(f: &mut std::fmt::Formatter<'_>,
                  quote: char,
                  s: &KString,
                  quote_required: bool,
                  colon_before: bool,
                  colon_after: bool)
                  -> Result<(), std::fmt::Error> {
    if s.is_empty() {
        f.write_fmt(format_args!("{}{}", quote, quote))
    } else {
        let mut out = String::new();
        // ^ XX oh I thought I could share it. And do need tmp (can't
        // just output everything via f directly) in case of
        // !quote_required (or would need 2 passes).
        let mut need_quote = quote_required;
        for c in s.chars() {
            if c == quote || c == '\\' {
                out.push('\\');
                out.push(c);
                need_quote = true;
            } else {
                out.push(c);
                if ! c.is_ascii_alphanumeric() {
                    need_quote = true;
                }
            } 
        }
        if colon_before {
            f.write_char(':')?
        }
        if need_quote {
            f.write_fmt(format_args!("{}{}{}", quote, out, quote))?
        } else {
            f.write_str(&out)?
        }
        if colon_after {
            f.write_char(':')?
        }
        Ok(())
    }
}



// XX these must be configurable in the future
// R7RS:

pub fn char2name(c: char) -> Option<&'static str> {
    match c {
        '\x07' => Some("alarm"),
        '\x08' => Some("backspace"),
        '\x7F' => Some("delete"),
        '\x1B' => Some("escape"),
        '\n' => Some("newline"),
        '\0' => Some("null"),
        '\r' => Some("return"),
        ' ' => Some("space"),
        '\t' => Some("tab"),
        _ => None
    }
}
pub fn name2char(s: &str) -> Option<char> {
    match s {
        "alarm" => Some('\x07'),
        "backspace" => Some('\x08'),
        "delete" => Some('\x7F'),
        "escape" => Some('\x1B'),
        "newline" => Some('\n'),
        "null" => Some('\0'),
        "return" => Some('\r'),
        "space" => Some(' '),
        "tab" => Some('\t'),
        _ => None
    }
}


impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            Atom::Bool(b) => f.write_fmt(format_args!("#{}", if *b { "t" } else { "f" })),
            Atom::Char(c) => {
                f.write_str("#\\")?;
                if let Some(name) = char2name(*c) {
                    f.write_str(name)
                } else {
                    f.write_char(*c)
                }
            }
            Atom::String(s) => fmt_stringlike(f, '"', s, true, false, false),
            Atom::Symbol(s) => fmt_stringlike(f, '|', s, false, false, false),
            Atom::Keyword1(s) => fmt_stringlike(f, '|', s, false, true, false), // :foo
            Atom::Keyword2(s) => fmt_stringlike(f, '|', s, false, false, true), // foo:
            Atom::Number(n) => n.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parenkind {
    Round,
    Square,
    Curly
}

impl Parenkind {
    pub fn opening(self) -> char {
        match self {
            Parenkind::Round => '(',
            Parenkind::Square => '[',
            Parenkind::Curly => '{'
        }
    }
    pub fn closing(self) -> char {
        match self {
            Parenkind::Round => ')',
            Parenkind::Square => ']',
            Parenkind::Curly => '}'
        }
    }
}

// Vec-based version of values
#[derive(Debug)]
pub enum VValue {
    Atom(Atom),
    List(Parenkind, bool, Vec<VValue>), // bool: true = improper list
}

impl std::fmt::Display for VValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            VValue::Atom(t) => {
                t.fmt(f)
            }
            VValue::List(pk, impr, v) => {
                f.write_char(pk.opening())?;
                let len = v.len();
                for (i, item) in v.iter().enumerate() {
                    item.fmt(f)?;
                    if i + 2 < len {
                        f.write_char(' ')?;
                    } else if i + 1 < len {
                        if *impr {
                            f.write_str(" . ")?;
                        } else {
                            f.write_char(' ')?;
                        }
                    }
                }
                f.write_char(pk.closing())
            }
        }
    }
}

