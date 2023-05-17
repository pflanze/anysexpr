// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Holding the static information about the source or sink of a
//! stream (i.e. other than position).

// (This might also be called Container. SourceContainer is not OK
// since it might going to be used for sinks, too.)

use crate::pos::Pos;
use std::{path::PathBuf, fmt::{Formatter, Debug, Display}};

pub trait Context : Debug + Send + Sync {
    /// Format location to be put *after* the error reason and a
    /// space, includes "in" or "from" or "to".
    fn format_with_pos(&self, p: Pos, f: &mut Formatter<'_>)
                       -> Result<(), std::fmt::Error>;
    /// Format location to be put *before* a colon and the error
    /// reason. Does not include the colon.
    fn format_without_pos(&self, f: &mut Formatter<'_>)
                          -> Result<(), std::fmt::Error>;
    /// Same as `format_without_pos` but as a string.
    fn to_string_without_pos(&self) -> String {
        format!("{}", &Helper(self))
    }
}

// Hack to get access to a Formatter, since Formatter::new is
// inaccessible:
struct Helper<'t, T: Context + ?Sized>(&'t T);
impl<'t, T: Context + ?Sized> Display for Helper<'t, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.0.format_without_pos(f)
    }
}


#[derive(Debug)]
pub struct FileContext {
    pub path: PathBuf
}

impl Context for FileContext {
    fn format_with_pos(&self, pos: Pos, f: &mut Formatter<'_>)
                       -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("in {:?}{}",
                                 &self.path,
                                 pos))
    }
    fn format_without_pos(&self, f: &mut Formatter<'_>)
                          -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("{:?}",
                                 &self.path))
    }
}

#[derive(Debug)]
pub struct SpecialContext {
    name: String
}

impl Context for SpecialContext {
    fn format_with_pos(&self, pos: Pos, f: &mut Formatter<'_>)
                       -> Result<(), std::fmt::Error> {
        // XX or might be `to` in the future? Take a direction
        // argument, or expect the caller to add the from/to?
        f.write_fmt(format_args!("from ({}){}",
                                 &self.name,
                                 pos))
    }
    fn format_without_pos(&self, f: &mut Formatter<'_>)
                       -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("({})",
                                 &self.name))
    }
}
