// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utilities for debugging the anysexpr library

use num::BigInt;

use crate::{value::{VValue, Atom, Parenkind, symbol, VValueWithPos}, number::R5RSNumber, pos::Pos};

fn listlike(
    pk: Parenkind,
    improper: bool,
    vals: Vec<VValueWithPos>,
    pos: Pos
) -> VValueWithPos {
    let mut vals2 : Vec<VValueWithPos> = Vec::new();
    vals2.push(symbol(if improper {"improper-list"} else {"list"}).at(pos));
    for v in vals {
        vals2.push(v);
    }
    VValue::List(pk, false, vals2).at(pos)
}

fn list2(
    symname: &str,
    a: Atom,
    pos: Pos,
) -> VValueWithPos {
    let mut vals : Vec<VValueWithPos> = Vec::new();
    vals.push(symbol(symname).at(pos));
    vals.push(VValue::Atom(a).at(pos));
    VValue::List(Parenkind::Round, false, vals).at(pos)
}

fn listn(
    symname: &str,
    atoms: impl Iterator<Item=Atom>,
    pos: Pos
) -> VValueWithPos {
    let mut vals : Vec<VValueWithPos> = Vec::new();
    vals.push(symbol(symname).at(pos));
    for a in atoms {
        vals.push(VValue::Atom(a).at(pos)); // XX huh losing information here
    }
    VValue::List(Parenkind::Round, false, vals).at(pos)
}

fn integer(n: u32) -> Atom {
    Atom::Number(R5RSNumber::Integer(BigInt::from(n)))
}

fn chars2atoms(cs: impl Iterator<Item=char>) -> impl Iterator<Item=Atom> {
    cs.map(|c| integer(c as u32))
}

impl VValueWithPos {
    pub fn dump(&self) -> VValueWithPos {
        let VValueWithPos(val, pos) = self;
        match val {
            VValue::Atom(a) => match a {
                Atom::Bool(b) =>
                    symbol(if *b { "true" } else { "false" }).at(*pos),
                Atom::Char(c) => list2("integer->char", integer(*c as u32), *pos),
                Atom::Keyword1(s) => listn("keyword1", chars2atoms(s.chars()), *pos),
                Atom::Keyword2(s) => listn("keyword2", chars2atoms(s.chars()), *pos),
                Atom::String(s) => listn("string", chars2atoms(s.chars()), *pos),
                Atom::Symbol(s) => listn("symbol", chars2atoms(s.chars()), *pos),
                Atom::Number(_) => list2("number", a.clone(), *pos), //X ?
            }
            VValue::List(pk, improper, vals) => {
                listlike(*pk, *improper, vals.iter().map(|v| v.dump()).collect(), *pos)
            }
        }
    }
}

