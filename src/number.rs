// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A representation of the number types possible in S-expressions
//! (numeric tower).

use num::{BigInt, rational::Ratio};

/// TODO: complex numbers, inexact reals
#[derive(Debug, Clone, PartialEq)]
pub enum R5RSNumber {
    // Complex(Box<R5RSNumber>, Box<R5RSNumber>),
    // Real(f64),
    Rational(Box<Ratio<BigInt>>),
    // ^ boxing since BigInt is Vec (RawVec (ptr and usize) and usize)
    //   plus sign. Proper optimization later, though.
    Integer(BigInt)
}

impl std::fmt::Display for R5RSNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            R5RSNumber::Rational(n) =>
                f.write_fmt(format_args!("{}/{}", n.numer(), n.denom())),
            R5RSNumber::Integer(n) => f.write_fmt(format_args!("{}", n)),
        }
    }
}

