// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This is an S-Expression parser and formatter with the following goals:
//! 
//! * Offering direct access to the tokenizer, `anysexpr::parse`, but also
//!   `anysexpr::read` to build an in-memory tree easily.
//! 
//! * Good error reporting (precise location information and
//!   messages).
//! 
//! * (Future) Make the data constructors for [anysexpr::read](crate::read)
//!   parametrizable (generic), e.g. like in the `sexpr_parser` crate.
//! 
//! * Streaming: allow to read from and print to file handles lazily,
//!   for use e.g. in communications. This currently works by using
//!   [anysexpr::parse](crate::parse) directly for input, or creating
//!   tokens to print via a custom loop for output. Future: more
//!   possibilities, e.g. turn a tree into a token stream, or
//!   parameterize with a tree that's generated on demand while
//!   printing.
//! 
//! * (Future) Support various s-expression variants (R*RS, Guile, Clojure,
//!   Common Lisp, ..) via runtime (and compile-time?) settings.
//! 
//! * (Perhaps) be usable on microcontrollers (small code, no-std?).
//! 
//! The author is quite new to Rust. There will be API guideline entries
//! not currently being followed, help in that area is as welcome as in
//! other areas.

pub mod buffered_chars; // although this is a hack
pub mod context;
pub mod number;
pub mod parse;
pub mod pos;
pub mod read;
pub mod settings;
pub mod value;
pub mod debug;
