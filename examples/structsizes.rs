// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Not an example, but a program to show the struct sizes for
//! possible optimization.

use anysexpr::{settings::{AnysexprFormat, Modes, Settings, GAMBIT_FORMAT}, context::{FileContext, SpecialContext}, parse::{ParseErrorWithPos, TokenWithPos, Token, ParseError, parse}, pos::Pos, read::{ReadErrorWithPos, ReadErrorWithContext, ReadErrorWithLocation, ReadError}, value::{VValue, VValueWithPos, SpecialKind, Atom, Parenkind}, number::{R5RSNumber, Integer, Rational}, buffered_chars::buffered_chars};
use kstring::KString;
use num::BigInt;

fn pr(ctx: &str, nam: &str, siz: usize) {
    println!("{siz}\t{ctx}\t{nam}")
}

const FQTY : bool = false;

macro_rules! ctx {
    ( $ctx:expr ) => {
        macro_rules! p {
            ( $t:ty ) => {
                let typename =
                    if FQTY {
                        std::any::type_name::<$t>()
                    } else {
                        stringify!($t)
                    };
                pr($ctx, typename, std::mem::size_of::<$t>())
            }
        }
    }
}

// And, fun, get the return type size of a function:
fn sz<R, F: FnOnce() -> R>(
    _f: F
) -> usize {
    std::mem::size_of::<R>()
}


fn main() {
    {
        ctx!("context");
        p!{FileContext};
        p!{SpecialContext};
        p!{Pos};
    }

    {
        ctx!("settings");
        p!{AnysexprFormat};
        p!{Modes};
        p!{Settings};
    }
    
    {
        ctx!("value");
        p!{BigInt};
        p!{Integer};
        p!{Rational};
        p!{R5RSNumber};
        p!{KString};
        p!{SpecialKind};
        p!{Atom};
        p!{VValue};
        p!{VValueWithPos};
    }

    {
        ctx!("parse");
        p!{Token};
        p!{TokenWithPos};
        p!{ParseError};
        p!{ParseErrorWithPos};

        p!{Result<(u32, Option<(char, Pos)>), ParseErrorWithPos>};
        p!{Result<(char, Option<(char, Pos)>), ParseErrorWithPos>};
        p!{Result<(Option<char>, Option<(char, Pos)>), ParseErrorWithPos>};
        // Item in impl Iterator<Item = Result<TokenWithPos, ParseErrorWithPos>> + 's:
        p!{Result<TokenWithPos, ParseErrorWithPos>};

        {
            // And, fun, the size of the return type of a fn:
            let settings = Settings {
                format: &GAMBIT_FORMAT,
                modes: &Modes {
                    retain_comments: true,
                    retain_whitespace: true,
                    allow_improper_lists: true,
                }
            };
            pr("parse", "return type of `parse`", sz(|| {
                let cs = buffered_chars("hi".as_bytes());
                parse(cs, &settings)
            }));
        }
    }
    
    {
        ctx!("read");
        p!{std::io::Error};
        p!{&'static str};
        p!{&&'static str};
        p!{Box<&'static str>};
        p!{(Parenkind, Pos, Parenkind)};
        p!{ReadError};
        p!{ReadErrorWithPos};
        p!{ReadErrorWithContext};
        p!{ReadErrorWithLocation};
        
        p!{Result<Option<VValueWithPos>, ReadErrorWithPos>};
        p!{Result<(Vec<VValueWithPos>, Option<Pos>), ReadErrorWithPos>};
        p!{Result<Vec<VValueWithPos>, ReadErrorWithPos>};
        p!{Result<Vec<VValueWithPos>, ReadErrorWithLocation>};
    }

}
