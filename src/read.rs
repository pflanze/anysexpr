// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The implementation of the lisp `read` function (as well as helpers
//! around it), i.e. parsing a character stream to an S-expression
//! tree representation. See [parse](crate::parse) for using the
//! underlying tokenizer directly.

use crate::pos::Pos;
use crate::context::{self, Context};
use crate::parse::{Token, TokenWithPos, parse,
                   ParseError, ParseErrorWithPos};
use crate::settings::{Settings, Modes, GAMBIT_FORMAT};
use crate::value::{VValue, Parenkind, symbol, list2, VValueWithPos};
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
    #[error("'.' outside of list context")]
    DotOutsideListContext,
    #[error("improperly placed '.'")]
    ImproperlyPlacedDot,
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
    #[error("missing expression after {0}")]
    // MissingExpressionAfter(Token), // XX large because of Token, right?
    MissingExpressionAfter(&'static str),
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

trait At<T> {
    fn at(self, p: Pos) -> Result<T, ReadErrorWithPos>;
}

impl<T> At<T> for Result<T, ReadError> {
    fn at(self, p: Pos) -> Result<T, ReadErrorWithPos> {
        match self {
            Err(e) => Err(e.at(p)),
            Ok(v) => Ok(v)
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

fn dec(fuel: u32) -> Result<u32, ReadError> {
    if fuel == 0 {
        return Err(ReadError::NestingTooDeep)
    }
    Ok(fuel - 1)
}


// Reads one expression. Returns None on EOF. Signals
// ReadError::UnexpectedClosingParen if there's no expression left in
// the current level.
fn iterator_read(
    ts: &mut impl Iterator<Item = Result<TokenWithPos, ParseErrorWithPos>>,
    depth_fuel: u32,
) -> Result<Option<VValueWithPos>, ReadErrorWithPos>
{
    let get_prefixing =
        |ts, quotepos, symname| -> Result<Option<VValueWithPos>, ReadErrorWithPos> {
            if let Some(expr) = iterator_read(ts, dec(depth_fuel).at(quotepos)?)? {
                Ok(Some(list2(symbol(symname).at(quotepos), expr).at(quotepos)))
            } else {
                Err(ReadError::MissingExpressionAfter(symname).at(quotepos))
            }
        };
    while let Some(TokenWithPos(t, pos)) = ts.next().transpose()? {
        match t {
            Token::Dot => {
                return Err(ReadError::ImproperlyPlacedDot.at(pos))
            }
            Token::Quote => {
                return get_prefixing(ts, pos, "quote")
            }
            Token::Quasiquote => {
                return get_prefixing(ts, pos, "quasiquote")
            }
            Token::Unquote => {
                return get_prefixing(ts, pos, "unquote")
            }
            Token::UnquoteSplicing => {
                return get_prefixing(ts, pos, "unquote-splicing")
            }
            Token::Whitespace(_) => {}
            Token::Comment(_, _) => {}
            Token::Open(pk) => {
                let (e, maybedot) =
                    iterator_read_all(ts, Some((pk, pos)), dec(depth_fuel).at(pos)?)?;
                return Ok(Some(VValue::List(pk, maybedot.is_some(), e).at(pos)))
            }
            Token::Close(pk) => {
                return Err(ReadError::UnexpectedClosingParen(pk).at(pos))
            }
            Token::Atom(a) => {
                return Ok(Some(VValue::Atom(a).at(pos)));
            }
        }        
    }
    Ok(None)
}

// Read and fill a vector of values up to the expected end paren, and
// return the vector and the position of a Dot, if any. Checking
// whether a dot is allowed is left to the caller. The check whether
// the right number of items before and after the dot appeared is done
// by iterator_read_all.
fn iterator_read_all(
    ts: &mut impl Iterator<Item = Result<TokenWithPos, ParseErrorWithPos>>,
    opt_parenkind: Option<(Parenkind, Pos)>,
    depth_fuel: u32,
) -> Result<(Vec<VValueWithPos>, Option<Pos>), ReadErrorWithPos>
{
    let mut vs = Vec::new();
    let mut seen_dot: Option<(Pos, usize)> = None;
    let result = |seen_dot, vs: Vec<VValueWithPos>| {
        if let Some((dotpos, i)) = seen_dot {
            let n_items_after_dot = vs.len() - i;
            match n_items_after_dot {
                1 => return Ok((vs, Some(dotpos))),
                0 => Err(ReadError::MissingItemAfterDot.at(dotpos)),
                _ => Err(ReadError::ExpectingOneItemAfterDot(n_items_after_dot)
                         .at(dotpos)),
            }
        } else {
            return Ok((vs, None));
        }
    };
    while let Some(r) = iterator_read(ts, depth_fuel).transpose() {
        match r {
            Err(ep) => {
                let ReadErrorWithPos { err, pos } = &ep;
                match err {
                    ReadError::IO(_) => return Err(ep),
                    ReadError::ImproperlyPlacedDot => {
                        if let Some((oldpos, _)) = seen_dot {
                            return Err(ReadError::DotAlreadyAppeared(oldpos).at(*pos))
                        } else {
                            let i = vs.len();
                            if i == 0 {
                                return Err(ReadError::DotWithoutPrecedingItem.at(*pos))
                            }
                            seen_dot = Some((*pos, i));
                        }
                    }
                    ReadError::UnexpectedClosingParen(pk) => {
                        if let Some((parenkind, startpos)) = opt_parenkind {
                            if *pk == parenkind {
                                return result(seen_dot, vs)
                            } else {
                                return Err(ReadError::ParenMismatch(
                                    parenkind, startpos, *pk)
                                           .at(*pos))
                            }
                        } else {
                            return Err(ep)
                        }
                    }
                    _ => return Err(ep)
                }
            }
            Ok(v) => {
                vs.push(v);
            }
        }
    }
    if let Some((parenkind, startpos)) = opt_parenkind {
        Err(ReadError::PrematureEofExpectingClosingParen(parenkind)
            .at(startpos))
    } else {
        result(seen_dot, vs)
    }
}

/// Read a single expression from an input stream. Returns None on
/// EOF. Signals ReadError::UnexpectedClosingParen if there's no
/// expression left in the current level.
pub fn read(
    fh: impl Read,
) -> Result<Option<VValueWithPos>, ReadErrorWithPos>
{
    let mut cs = buffered_chars(fh); // XXX must not buffer *here*!
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
    iterator_read(&mut ts, depth_fuel)
}

/// Read (deserialize) all of an input stream to a sequence
/// of [VValueWithPos](VValueWithPos).
pub fn read_all(
    fh: impl Read,
) -> Result<Vec<VValueWithPos>, ReadErrorWithPos>
{
    let mut cs = buffered_chars(fh); // XX should not buffer here!
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
    let (v, maybedot) = iterator_read_all(
        &mut ts,
        None,
        depth_fuel)?;
    if let Some(pos) = maybedot {
        Err(ReadError::DotOutsideListContext.at(pos))
    } else {
        Ok(v)
    }
}

/// Read (deserialize) the contents of a file to a sequence of
/// [VValueWithPos](VValueWithPos).
pub fn read_file(path: &Path) -> Result<Vec<VValueWithPos>, ReadErrorWithLocation> {
    let fh = io_add_file(File::open(path), path)?;
    let v = rewp_add_file(read_all(fh), path)?;
    Ok(v)
}

/// Write (serialize) a sequence of [VValue](VValue) or
/// [VValueWithPos](VValueWithPos) to an output stream.
pub fn write_all<'t, T: Display + 't>(
    out: impl Write,
    vals: impl IntoIterator<Item = &'t T>
) -> Result<(), std::io::Error> {
    let mut out = out; // for `File`
    let mut seen_item = false;
    for v in vals.into_iter() {
        write!(out, "{}{}\n", if seen_item {"\n"} else {""}, v)?;
        seen_item = true;
    }
    Ok(())
}

/// Write (serialize) a sequence of [VValue](VValue) to a file.
pub fn write_file<'t>(path: &Path, vals: impl IntoIterator<Item = &'t VValue>)
                      -> Result<(), std::io::Error> {
    write_all(File::open(path)?, vals)
}

