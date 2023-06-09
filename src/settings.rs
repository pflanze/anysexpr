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
pub struct AnysexprFormat<'t> {
    pub name: &'t str,
    pub has_dotted_pairs: bool,
    pub octal_escapes_in_delimited: bool,
    pub x_escape_terminated_by_semicolon_in_delimited: bool,
    pub x_escape_len: u8,
    pub accept_long_false_true: bool,
    pub hashcolon_is_keyword: bool, // #:foo, keyword vs. uninterned symbol
}

pub const GAMBIT_FORMAT : AnysexprFormat = AnysexprFormat {
    name: "Gambit",
    has_dotted_pairs: true,
    octal_escapes_in_delimited: true,
    x_escape_terminated_by_semicolon_in_delimited: false,
    x_escape_len: 8,
    accept_long_false_true: false,
    hashcolon_is_keyword: false,
};

pub const R7RS_FORMAT : AnysexprFormat = AnysexprFormat {
    name: "R7RS",
    has_dotted_pairs: true,
    octal_escapes_in_delimited: false,
    x_escape_terminated_by_semicolon_in_delimited: true,
    x_escape_len: 8, // XX check
    accept_long_false_true: false, // XX check
    hashcolon_is_keyword: true, // XX check
};

pub const GUILE_FORMAT : AnysexprFormat = AnysexprFormat {
    name: "Guile",
    has_dotted_pairs: true,
    octal_escapes_in_delimited: false,
    x_escape_terminated_by_semicolon_in_delimited: true, // ?
    x_escape_len: 2,
    accept_long_false_true: true,
    hashcolon_is_keyword: true,
};


#[derive(Debug)]
pub struct Modes {
    /// Whether to report whitespace from `parse` (currently always
    /// dropped in `read`).
    pub retain_whitespace: bool,
    /// Whether to report comments from `parse` (currently always
    /// dropped in `read`).
    pub retain_comments: bool,
    /// `false` => disallow `(a . b)` where b is not a proper list.
    /// `(a . (b))` is still allowed if the format supports the
    /// syntax.
    pub allow_improper_lists: bool,
}

#[derive(Debug)]
pub struct Settings<'t> {
    pub format: &'t AnysexprFormat<'t>,
    pub modes: &'t Modes,
}

