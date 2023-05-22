// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utilities for debugging the anysexpr library

use kstring::KString;
use num::BigInt;

use crate::{value::{VValue, Atom, Parenkind}, number::R5RSNumber};

fn symbol(s: &str) -> VValue {
    VValue::Atom(Atom::Symbol(KString::from_ref(s)))
}

fn listlike(
    pk: Parenkind,
    improper: bool,
    vals: Vec<VValue>
) -> VValue {
    let mut vals2 : Vec<VValue> = Vec::new();
    vals2.push(symbol(if improper {"improper-list"} else {"list"}));
    for v in vals {
        vals2.push(v);
    }
    VValue::List(pk, false, vals2)
}

fn list2(symname: &str,
         a: Atom) -> VValue {
    let mut vals : Vec<VValue> = Vec::new();
    vals.push(symbol(symname));
    vals.push(VValue::Atom(a));
    VValue::List(Parenkind::Round, false, vals)
}

fn listn(symname: &str,
         atoms: impl Iterator<Item=Atom>) -> VValue {
    let mut vals : Vec<VValue> = Vec::new();
    vals.push(symbol(symname));
    for a in atoms {
        vals.push(VValue::Atom(a));
    }
    VValue::List(Parenkind::Round, false, vals)
}

fn integer(n: u32) -> Atom {
    Atom::Number(R5RSNumber::Integer(BigInt::from(n)))
}

fn chars2atoms(cs: impl Iterator<Item=char>) -> impl Iterator<Item=Atom> {
    cs.map(|c| integer(c as u32))
}

impl VValue {
    pub fn dump(&self) -> VValue {
        match self {
            VValue::Atom(a) => match a {
                Atom::Bool(b) => if *b { symbol("true") } else { symbol("false") },
                Atom::Char(c) => list2("integer->char", integer(*c as u32)),
                Atom::Keyword1(s) => listn("keyword1", chars2atoms(s.chars())),
                Atom::Keyword2(s) => listn("keyword2", chars2atoms(s.chars())),
                Atom::String(s) => listn("string", chars2atoms(s.chars())),
                Atom::Symbol(s) => listn("symbol", chars2atoms(s.chars())),
                Atom::Number(_) => list2("number", a.clone()), //X ?
            }
            VValue::List(pk, improper, vals) => {
                listlike(*pk, *improper, vals.iter().map(|v| v.dump()).collect())
            }
        }
    }
}
