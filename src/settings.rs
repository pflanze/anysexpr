// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Settings for both reading (parsing) and writing (serializing)
//! data.

#[derive(Debug)]
pub struct Format {
    octal_escapes_in_delimited: bool,
}

pub const GAMBIT_FORMAT : Format = Format {
    octal_escapes_in_delimited: true,
};


#[derive(Debug)]
pub struct Modes {
    pub retain_whitespace: bool,
    pub retain_comments: bool,
}

#[derive(Debug)]
pub struct Settings {
    pub format: Box<Format>,
    pub modes: Box<Modes>,
}

