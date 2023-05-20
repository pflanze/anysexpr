// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cmp::Eq;

/// Both line and col are zero based; Emacs uses 1-based line
/// numbering, so line is incremented by 1 in Display.

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        // This, when prefixed with a Debug style path string, is
        // following the Emacs convention for location information.
        f.write_fmt(format_args!("@{}.{}", self.line + 1, self.col))
    }
}

