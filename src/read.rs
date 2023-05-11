use crate::pos::Pos;
use crate::parse::{Token, TokenWithPos, ParseError, ParseSettings, parse};
use crate::value::{Atom, VValue, Parenkind};
use crate::buffered_chars::buffered_chars;
use std::path::Path;
use kstring::KString;
use anyhow::{Result, bail};
use std::fs::File;


fn slurp(
    locator: &dyn Fn(Pos) -> String,
    ts: &mut impl Iterator<Item = Result<TokenWithPos,
                                         ParseError>>,
    opt_parenkind: Option<(Parenkind, Pos)>)
    -> Result<Vec<VValue>>
{
    let mut v = Vec::new();
    let mut current_keyword2: Option<KString> = None;
    let mut seen_dot: Option<Pos> = None;
    while let Some(te) = ts.next() {
        let TokenWithPos(t, pos) = te?;
        match t {
            Token::Dot => {
                if let Some(oldpos) = seen_dot {
                    bail!("dot already appeared {}, again {}",
                          locator(oldpos),
                          locator(pos))
                } else {
                    seen_dot = Some(pos);
                    // XXX check that it is followed by another item!
                }
            }
            Token::Quote => {
                todo!()
            }
            Token::Quasiquote => {
                todo!()
            }
            Token::Unquote => {
                todo!()
            }
            Token::Whitespace(_) => {}
            Token::Comment(_, _) => {}
            Token::Open(pk) => {
                let e = slurp(locator, ts, Some((pk, pos)))?;
                v.push(VValue::List(pk,
                                    seen_dot.is_some(),
                                    e));
            }
            Token::Close(pk) => {
                if let Some((parenkind, startpos)) = opt_parenkind {
                    if pk == parenkind {
                        if let Some(kw) = current_keyword2 {
                            bail!("expected value after keyword `{}` {}",
                                  Atom::Keyword2(kw),
                                  locator(pos))
                        } else {
                            return Ok(v);
                        }
                    } else {
                        bail!("'{}' {} expects '{}', got '{}' {}",
                              parenkind.opening(),
                              locator(startpos),
                              parenkind.closing(),
                              pk.closing(),
                              locator(pos))
                    }
                } else {
                    bail!("got closing '{}' though none expected {}",
                          pk.closing(),
                          locator(pos))
                }
            }
            Token::Atom(a) => {
                match a {
                    Atom::Keyword1(_)  => bail!("unimplemented"),
                    Atom::Keyword2(ref s) => {
                        if let Some(oldkw2) = current_keyword2 {
                            // XX should this be allowed?
                            bail!("keyword2 `{}` followed by another \
                                   keyword2: `{}` {}",
                                  Atom::Keyword2(oldkw2), // feels hacky?
                                  a,
                                  locator(pos));
                        } else {
                            current_keyword2 = Some(s.clone());
                        }
                    }
                    _ => {
                        if let Some(kw) = current_keyword2 {
                            v.push(VValue::KeyValue(
                                kw,
                                Box::new(VValue::Atom(a)))); // XXX not a pls?
                            // ^ XXX should get rec thing  generally   here
                            current_keyword2 = None;
                        } else {
                            v.push(VValue::Atom(a));
                        }
                    }
                }
            }
        }
    }
    if let Some((parenkind, startpos)) = opt_parenkind {
        bail!("premature EOF while expecting closing paren '{}' (opening {})",
              parenkind.closing(),
              locator(startpos))
    } else {
        Ok(v)
    }
}

pub fn read(
    path: &Path,
    fh: File,
) -> Result<Vec<VValue>> {
    let mut cs = buffered_chars(fh);
    let settings = ParseSettings {
        furnish_whitespace: false,
        furnish_comments: false,
    };
    let mut ts = parse(&mut cs, settings);
    slurp(
        &|pos| format!("at {path:?}{pos}"),
        &mut ts, None)
}

pub fn read_file(path: &Path) -> Result<Vec<VValue>> {
    let fh = File::open(path)?;
    read(path, fh)
}
