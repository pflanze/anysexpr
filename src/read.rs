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
use crate::settings::{Settings, Modes, AnysexprFormat};
use crate::value::{VValue, Parenkind, symbol, list2, VValueWithPos};
use crate::buffered_chars::buffered_chars;
use std::fmt::{Formatter, Display, Debug};
use std::io::{Write, BufReader};
use std::path::Path;
use std::fs::File;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("{0}")]
    PE(Box<ParseError>),
    #[error("{0}")]
    IO(std::io::Error),
    #[error("missing item after '.'")]
    MissingItemAfterDot,
    #[error("expecting exactly one item after '.'")]
    ExpectingOneItemAfterDot,
    #[error("'.' without preceding item")]
    DotWithoutPrecedingItem,
    #[error("'.' outside of list context")]
    DotOutsideListContext,
    #[error("'.' only allowed in (..) lists, but used in {}..{}",
            .0.opening(), .0.closing())]
    DotInWrongListContext(Parenkind),
    #[error("improperly placed '.'")]
    ImproperlyPlacedDot,
    #[error("improper lists disallowed in given mode")]
    ImproperListsNotAllowedByMode,
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
    MissingExpressionAfter(Box<&'static str>),
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
            err: ReadError::PE(Box::new(err)),
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


pub trait TokensRead<T: Iterator<Item = Result<TokenWithPos, ParseErrorWithPos>>> {

    /// Read one expression. Returns None on EOF. Signals
    /// ReadError::UnexpectedClosingParen if there's no expression left in
    /// the current level.
    fn read(
        &mut self,
        depth_fuel: u32,
        modes: &Modes,
    ) -> Result<Option<VValueWithPos>, ReadErrorWithPos>;

    /// Read and fill a vector of values up to the expected end paren, and
    /// return the vector and the position of a Dot, if any. Checking
    /// whether a dot is allowed is left to the caller.
    fn read_all(
        &mut self,
        opt_parenkind: Option<(Parenkind, Pos)>,
        depth_fuel: u32,
        modes: &Modes,
    ) -> Result<(Vec<VValueWithPos>, Option<Pos>), ReadErrorWithPos>;
}


impl<T: Iterator<Item = Result<TokenWithPos, ParseErrorWithPos>>> TokensRead<T> for T {

    fn read(
        &mut self,
        depth_fuel: u32,
        modes: &Modes,
    ) -> Result<Option<VValueWithPos>, ReadErrorWithPos>
    {
        let get_prefixing =
            |ts: &mut T, quotepos, symname| ->
            Result<Option<VValueWithPos>, ReadErrorWithPos> {
                if let Some(expr) = ts.read(dec(depth_fuel).at(quotepos)?, modes)? {
                    Ok(Some(list2(symbol(symname).at(quotepos), expr).at(quotepos)))
                } else {
                    Err(ReadError::MissingExpressionAfter(Box::new(symname))
                        .at(quotepos))
                }
            };
        while let Some(TokenWithPos(t, pos)) = self.next().transpose()? {
            match t {
                Token::Dot => {
                    return Err(ReadError::ImproperlyPlacedDot.at(pos))
                }
                Token::Quote => {
                    return get_prefixing(self, pos, "quote")
                }
                Token::Quasiquote => {
                    return get_prefixing(self, pos, "quasiquote")
                }
                Token::Unquote => {
                    return get_prefixing(self, pos, "unquote")
                }
                Token::UnquoteSplicing => {
                    return get_prefixing(self, pos, "unquote-splicing")
                }
                Token::Whitespace(_) => {}
                Token::CommentExpr => {
                    // read and ignore the next expression
                    self.read(dec(depth_fuel).at(pos)?, modes)?;
                }
                Token::Comment(_, _) => {}
                Token::Open(pk) => {
                    let (e, maybedot) =
                        self.read_all(Some((pk, pos)), dec(depth_fuel).at(pos)?, modes)?;
                    if maybedot.is_some() && !modes.allow_improper_lists {
                        return Err(ReadError::ImproperListsNotAllowedByMode.at(maybedot.unwrap()))
                    }
                    return Ok(Some(VValue::List(pk, maybedot, e).at(pos)))
                }
                Token::Close(pk) => {
                    return Err(ReadError::UnexpectedClosingParen(pk).at(pos))
                }
                Token::Atom(a) => {
                    return Ok(Some(VValue::Atom(a).at(pos)))
                }
            }        
        }
        Ok(None)
    }
    
    fn read_all(
        &mut self,
        opt_parenkind: Option<(Parenkind, Pos)>,
        depth_fuel: u32,
        modes: &Modes,
    ) -> Result<(Vec<VValueWithPos>, Option<Pos>), ReadErrorWithPos>
    {
        let mut vs = Vec::new();
        let on_eof = |vs| {
            if let Some((parenkind, startpos)) = opt_parenkind {
                Err(ReadError::PrematureEofExpectingClosingParen(parenkind)
                    .at(startpos))
            } else {
                Ok((vs, None))
            }
        };
        while let Some(r) = self.read(depth_fuel, modes).transpose() {
            match r {
                Err(ep) => {
                    let ReadErrorWithPos { err, pos } = &ep;
                    match err {
                        ReadError::IO(_) => return Err(ep),
                        ReadError::ImproperlyPlacedDot => {
                            if let Some((pk, _pos)) = opt_parenkind {
                                if pk != Parenkind::Round {
                                    return Err(ReadError::DotInWrongListContext(pk)
                                               .at(*pos))
                                }
                            }
                            if vs.len() == 0 {
                                return Err(ReadError::DotWithoutPrecedingItem.at(*pos))
                            }
                            if let Some(vp) = self.read(dec(depth_fuel).at(*pos)?,
                                                        modes)? {
                                // The next token must be a Close if we're
                                // in a list, or none otherwise:
                                let expecting_close = |ts: &mut T, result| {
                                    // Use token_read or get just one
                                    // token? Just one token: be lazy /
                                    // report the error *here* not some
                                    // later one.
                                    // XX this is copying much of the end
                                    // paren check logic further down,
                                    // sigh.
                                    if let Some(TokenWithPos(t, pos)) =
                                        ts.next().transpose()?
                                    {
                                        match t {
                                            Token::Close(pk_end) => {
                                                if let Some((pk, openpos)) = opt_parenkind {
                                                    if pk_end == pk {
                                                        Ok(result)
                                                    } else {
                                                        Err(
                                                            ReadError::ParenMismatch(
                                                                pk, openpos, pk_end)
                                                            .at(pos))
                                                    }
                                                } else {
                                                    Err(
                                                        ReadError::UnexpectedClosingParen(
                                                            pk_end).at(pos))
                                                }
                                            }
                                            _ => {
                                                Err(ReadError::ExpectingOneItemAfterDot
                                                    .at(pos))
                                            }
                                        }
                                    } else {
                                        if let Some((pk, openpos)) = opt_parenkind {
                                            Err(ReadError::PrematureEofExpectingClosingParen(
                                                pk).at(openpos))
                                        } else {
                                            Ok(result)
                                        }
                                    }
                                };
                                match vp.0 {
                                    VValue::Atom(_) => {
                                        vs.push(vp);
                                        return expecting_close(self, (vs, Some(*pos)))
                                    },
                                    VValue::List(pk1, improper1, mut vs1) => {
                                        // Perform "tail syntax
                                        // optimization" if it's the same
                                        // kind of list, ehr, also the
                                        // Round kind (we already checked
                                        // above that the context is
                                        // Round)
                                        if pk1 == Parenkind::Round {
                                            vs.append(&mut vs1);
                                            // Whether the current list
                                            // context is proper now
                                            // depends on whether vs1 was.
                                            return expecting_close(self, (vs, improper1))
                                        }
                                        // Otherwise keep nested
                                        vs.push(VValue::List(pk1, improper1, vs1)
                                                .at(vp.1));
                                        return expecting_close(self, (vs, Some(*pos)))
                                    }
                                }
                            } else {
                                return on_eof(vs)
                            }
                        }
                        ReadError::UnexpectedClosingParen(pk) => {
                            if let Some((parenkind, startpos)) = opt_parenkind {
                                if *pk == parenkind {
                                    return Ok((vs, None))
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
        on_eof(vs)
    }
}

impl<'f> AnysexprFormat<'f> {

    /// Read a single expression from an input stream. Returns None on
    /// EOF. Signals ReadError::UnexpectedClosingParen if there's no
    /// expression left in the current level.
    pub fn read(
        &self,
        charswithpos: impl IntoIterator<Item = anyhow::Result<(char, Pos)>>,
        modes: &Modes,
    ) -> Result<Option<VValueWithPos>, ReadErrorWithPos>
    {
        let settings = Settings {
            format: self,
            modes,
        };
        let depth_fuel = 500;
        // ^ the limit with default settings on Linux is around 1200
        let mut ts = parse(charswithpos.into_iter(), &settings);
        ts.read(depth_fuel, settings.modes)
    }

    /// Read (deserialize) all of an input stream to a sequence
    /// of [VValueWithPos](VValueWithPos).
    pub fn read_all(
        &self,
        charswithpos: impl IntoIterator<Item = anyhow::Result<(char, Pos)>>,
        modes: &Modes,
    ) -> Result<Vec<VValueWithPos>, ReadErrorWithPos>
    {
        let settings = Settings {
            format: self,
            modes,
        };
        let depth_fuel = 500;
        // ^ the limit with default settings on Linux is around 1200
        let mut ts = parse(charswithpos.into_iter(), &settings);
        let (v, maybedot) = ts.read_all(
            None,
            depth_fuel,
            settings.modes)?;
        if let Some(pos) = maybedot {
            Err(ReadError::DotOutsideListContext.at(pos))
        } else {
            Ok(v)
        }
    }

    /// Read (deserialize) the contents of a file to a sequence of
    /// [VValueWithPos](VValueWithPos).
    pub fn read_file(
        &self,
        path: &Path,
        modes: &Modes,
    ) -> Result<Vec<VValueWithPos>, ReadErrorWithLocation> {
        let fh = io_add_file(File::open(path), path)?;
        let cs = buffered_chars(BufReader::new(fh));
        let v = rewp_add_file(self.read_all(cs, modes), path)?;
        Ok(v)
    }

    /// Write (serialize) a [VValue](VValue) or
    /// [VValueWithPos](VValueWithPos) to an output stream.
    pub fn write<'t, T: Display + 't>(
        &self,
        out: &mut impl Write,
        val: &'t T
    ) -> Result<(), std::io::Error> {
        write!(out, "{}", val)
    }

    /// Write (serialize) a [VValue](VValue) or
    /// [VValueWithPos](VValueWithPos) and a newline to an output stream.
    pub fn writeln<'t, T: Display + 't>(
        &self,
        out: &mut impl Write,
        val: &'t T
    ) -> Result<(), std::io::Error> {
        write!(out, "{}\n", val)
    }

    /// Write (serialize) a sequence of [VValue](VValue) or
    /// [VValueWithPos](VValueWithPos) to an output stream.
    pub fn write_all<'t, T: Display + 't>(
        &self,
        out: &mut impl Write,
        vals: impl IntoIterator<Item = &'t T>
    ) -> Result<(), std::io::Error> {
        let mut seen_item = false;
        for v in vals.into_iter() {
            if seen_item {
                write!(out, "\n")?;
            }
            self.writeln(out, v)?;
            seen_item = true;
        }
        Ok(())
    }

    /// Write (serialize) a sequence of [VValue](VValue) to a file.
    pub fn write_file<'t>(
        &self,
        path: &Path,
        vals: impl IntoIterator<Item = &'t VValue>
    ) -> Result<(), std::io::Error> {
        self.write_all(&mut File::open(path)?, vals)
    }

}
