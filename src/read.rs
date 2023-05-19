// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::pos::Pos;
use crate::context::{self, Context};
use crate::parse::{Token, TokenWithPos, parse,
                   ParseError, ParseErrorWithPos};
use crate::settings::{Settings, Modes, GAMBIT_FORMAT};
use crate::value::{VValue, Parenkind};
use crate::buffered_chars::buffered_chars;
use std::fmt::{Formatter, Display, Debug};
use std::io::{Read, Write};
use std::path::Path;
use std::fs::File;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("{0}")]
    PE(ParseError),
    #[error("{0}")]
    IO(std::io::Error),
    #[error("missing item after '.'")]
    MissingItemAfterDot,
    #[error("expecting one item after '.', got {0}")]
    ExpectingOneItemAfterDot(usize),
    #[error("'.' already appeared {0}, again")]
    DotAlreadyAppeared(Pos),
    #[error("'.' without preceding item")]
    DotWithoutPrecedingItem,
    #[error("nesting too deep")]
    NestingTooDeep,
    #[error("'{}' {1} expects '{}', got '{}'",
            .0.opening(), .0.closing(), .2.closing())]
    ParenMismatch(Parenkind, Pos, Parenkind),
    #[error("unexpected closing character '{}'", .0.closing())]
    UnexpectedClosingParen(Parenkind),
    #[error("premature EOF while expecting closing character '{}' for '{}'",
            .0.closing(), .0.opening())]
    PrematureEofExpectingClosingParen(Parenkind),
    #[error("'.' outside of list context")]
    DotOutsideListContext
 }

#[derive(Error, Debug)]
#[error("{err} {pos}")]
pub struct ReadErrorWithPos {
    err: ReadError,
    pos: Pos
}

impl ReadError {
    fn at(self, p: Pos) -> ReadErrorWithPos {
        ReadErrorWithPos {
            err: self,
            pos: p
        }
    }
}

#[derive(Error, Debug)]
pub struct ReadErrorWithPosContext {
    err_with_pos: ReadErrorWithPos,
    container: Box<dyn Context>
}

impl Display for ReadErrorWithPosContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("{} ",
                                 self.err_with_pos.err))?;
        self.container.format_with_pos(self.err_with_pos.pos, f)?;
        Ok(())
    }
}

impl From<ParseErrorWithPos> for ReadErrorWithPos {
    fn from(ep: ParseErrorWithPos) -> ReadErrorWithPos {
        let ParseErrorWithPos { err, pos } = ep;
        ReadErrorWithPos {
            err: ReadError::PE(err),
            pos
        }
    }
}

#[derive(Error, Debug)]
pub enum ReadErrorWithContext {
    #[error("{}: {0}", .1.to_string_without_pos())]
    IO(std::io::Error, Box<dyn Context>)
}

#[derive(Error, Debug)]
pub enum ReadErrorWithLocation {
    #[error("{0}")]
    PC(Box<ReadErrorWithPosContext>),
    #[error("{0}")]
    IO(Box<ReadErrorWithContext>)
}


// XX change these to methods

// Transform an IO error without Pos context
fn io_add_file<T>(
    r: Result<T, std::io::Error>,
    path: &Path
) -> Result<T, ReadErrorWithLocation>
{
    match r {
        Err(e) => Err(ReadErrorWithLocation::IO(Box::new(
            ReadErrorWithContext::IO(
                e,
                Box::new(context::FileContext { path: path.to_path_buf() }))))),
        Ok(v) => Ok(v)
    }
}

// Transform ReadErrorWithPos adding file
fn rewp_add_file<T>(
    r: Result<T, ReadErrorWithPos>,
    path: &Path
) -> Result<T, ReadErrorWithLocation>
{
    match r {
        Err(e) => Err(ReadErrorWithLocation::PC(
            Box::new(
                ReadErrorWithPosContext {
                    err_with_pos: e,
                    container: Box::new(context::FileContext { path: path.to_path_buf() })
                }))),
        Ok(v) => Ok(v)
    }
}


// Read and fill a vector of values up to the expected end paren, and
// return the vector and the position of a Dot, if any. Checking
// whether a dot is allowed is left to the caller. The check whether
// the right number of items before and after the dot appeared is done
// by slurp.
fn slurp(
    ts: &mut impl Iterator<Item = Result<TokenWithPos,
                                         ParseErrorWithPos>>,
    opt_parenkind: Option<(Parenkind, Pos)>,
    depth_fuel: u32,
) -> Result<(Vec<VValue>, Option<Pos>), ReadErrorWithPos>
{
    let mut v = Vec::new();
    let mut seen_dot: Option<(Pos, usize)> = None;
    let result = |seen_dot, v: Vec<VValue>| {
        if let Some((dotpos, i)) = seen_dot {
            let n_items_after_dot = v.len() - i;
            match n_items_after_dot {
                1 => return Ok((v, Some(dotpos))),
                0 => Err(ReadError::MissingItemAfterDot.at(dotpos)),
                _ => Err(ReadError::ExpectingOneItemAfterDot(n_items_after_dot)
                         .at(dotpos)),
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
                    return Err(ReadError::DotAlreadyAppeared(oldpos).at(pos))
                } else {
                    let i = v.len();
                    if i == 0 {
                        return Err(ReadError::DotWithoutPrecedingItem.at(pos))
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
                    return Err(ReadError::NestingTooDeep.at(pos))
                }
                let (e, maybedot) = slurp(ts, Some((pk, pos)), depth_fuel - 1)?;
                v.push(VValue::List(pk,
                                    maybedot.is_some(),
                                    e));
            }
            Token::Close(pk) => {
                if let Some((parenkind, startpos)) = opt_parenkind {
                    if pk == parenkind {
                        return result(seen_dot, v)
                    } else {
                        return Err(ReadError::ParenMismatch(parenkind, startpos, pk)
                                   .at(pos))
                    }
                } else {
                    return Err(ReadError::UnexpectedClosingParen(pk)
                               .at(pos))
                }
            }
            Token::Atom(a) => {
                v.push(VValue::Atom(a));
            }
        }
    }
    if let Some((parenkind, startpos)) = opt_parenkind {
        Err(ReadError::PrematureEofExpectingClosingParen(parenkind)
            .at(startpos))
    } else {
        result(seen_dot, v)
    }
}

pub fn read_all(
    fh: impl Read,
) -> Result<Vec<VValue>, ReadErrorWithPos>
{
    let mut cs = buffered_chars(fh);
    let settings = Settings {
        format: &GAMBIT_FORMAT,
        modes: &Modes {
            retain_whitespace: false,
            retain_comments: false,
        },
    };
    let depth_fuel = 500;
    // ^ the limit with default settings on Linux is around 1200
    let mut ts = parse(&mut cs, &settings);
    let (v, maybedot) = slurp(
        &mut ts,
        None,
        depth_fuel)?;
    if let Some(pos) = maybedot {
        Err(ReadError::DotOutsideListContext.at(pos))
    } else {
        Ok(v)
    }
}

pub fn read_file(path: &Path) -> Result<Vec<VValue>, ReadErrorWithLocation> {
    let fh = io_add_file(File::open(path), path)?;
    let v = rewp_add_file(read_all(fh), path)?;
    Ok(v)
}

pub fn write_all<'t>(
    out: impl Write,
    vals: impl IntoIterator<Item = &'t VValue>
) -> Result<(), std::io::Error> {
    let mut out = out; // for `File`
    let mut seen_item = false;
    for v in vals.into_iter() {
        write!(out, "{}{}\n", if seen_item {"\n"} else {""}, v)?;
        seen_item = true;
    }
    Ok(())
}

pub fn write_file<'t>(path: &Path, vals: impl IntoIterator<Item = &'t VValue>)
                      -> Result<(), std::io::Error> {
    write_all(File::open(path)?, vals)
}

