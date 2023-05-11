// S-expr values (runtime data types)

use crate::number::R5RSNumber;
use std::fmt::Write;
use kstring::KString;

#[derive(Debug)]
pub enum Atom {
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

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
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

// Vec-based version of values (that also includes KeyValue)
#[derive(Debug)]
pub enum VValue {
    Atom(Atom),
    List(Parenkind, bool, Vec<VValue>), // bool: true = improper list
    KeyValue(KString, Box<VValue>),
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
            VValue::KeyValue(k, e) => {
                Atom::Keyword2(k.clone()).fmt(f)?;
                f.write_char(' ')?;
                e.fmt(f)
            }
        }
    }
}

