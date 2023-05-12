use crate::pos::Pos;
use crate::parse::{Token, TokenWithPos, ParseError, ParseSettings, parse};
use crate::value::{VValue, Parenkind};
use crate::buffered_chars::buffered_chars;
use std::path::Path;
use anyhow::{Result, bail};
use std::fs::File;


// Read and fill a vector of values up to the expected end paren, and
// return the vector and the position of a Dot, if any. Checking
// whether a dot is allowed is left to the caller. Checks whether the
// right number of items before and after the dot appeared is done by
// slurp.
fn slurp(
    locator: &dyn Fn(Pos) -> String,
    ts: &mut impl Iterator<Item = Result<TokenWithPos,
                                         ParseError>>,
    opt_parenkind: Option<(Parenkind, Pos)>,
    depth_fuel: u32,
) -> Result<(Vec<VValue>, Option<Pos>)>
{
    let mut v = Vec::new();
    let mut seen_dot: Option<(Pos, usize)> = None;
    let result = |seen_dot, v: Vec<VValue>| {
        if let Some((dotpos, i)) = seen_dot {
            let n_items_after_dot = v.len() - i;
            match n_items_after_dot {
                1 => return Ok((v, Some(dotpos))),
                0 => bail!("missing item after dot {}",
                           locator(dotpos)),
                _ => bail!("expecting one item after dot, got {} {}",
                           n_items_after_dot,
                           locator(dotpos)),
            }
        } else {
            return Ok((v, None));
        }
    };        
    while let Some(te) = ts.next() {
        let TokenWithPos(t, pos) = te?;
        match t {
            Token::Dot => {
                if let Some((oldpos, _)) = seen_dot {
                    bail!("dot already appeared {}, again {}",
                          locator(oldpos),
                          locator(pos))
                } else {
                    let i = v.len();
                    if i == 0 {
                        bail!("dot without preceding item {}",
                              locator(pos))
                    }
                    seen_dot = Some((pos, i));
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
                if depth_fuel == 0 {
                    bail!("nesting too deep {}", locator(pos))
                }
                let (e, maybedot) = slurp(locator, ts, Some((pk, pos)), depth_fuel - 1)?;
                v.push(VValue::List(pk,
                                    maybedot.is_some(),
                                    e));
            }
            Token::Close(pk) => {
                if let Some((parenkind, startpos)) = opt_parenkind {
                    if pk == parenkind {
                        return result(seen_dot, v)
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
                v.push(VValue::Atom(a));
            }
        }
    }
    if let Some((parenkind, startpos)) = opt_parenkind {
        bail!("premature EOF while expecting closing paren '{}' (opening {})",
              parenkind.closing(),
              locator(startpos))
    } else {
        return result(seen_dot, v)
    }
}

pub fn read(
    path: &Path,
    fh: File,
) -> Result<Vec<VValue>> {
    let mut cs = buffered_chars(fh);
    let settings = ParseSettings {
        whitespace: false,
        comments: false,
    };
    let depth_fuel = 500;
    // ^ limit with default settings on Linux is around 1200
    let mut ts = parse(&mut cs, settings);
    let locator = |pos| format!("at {path:?}{pos}");
    let (v, maybedot) = slurp(
        &locator,
        &mut ts,
        None,
        depth_fuel)?;
    if let Some(pos) = maybedot {
        bail!("dot outside list context {}",
              locator(pos))
    } else {
        Ok(v)
    }
}

pub fn read_file(path: &Path) -> Result<Vec<VValue>> {
    let fh = File::open(path)?;
    read(path, fh)
}
