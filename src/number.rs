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

use std::ops::{Mul, Add, Neg, Rem, Div};

use num::BigInt;

// XXX how does PartialOrd work here? OK?
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Integer {
    Small(i64),
    Big(Box<BigInt>)
}

impl std::fmt::Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            Integer::Small(i) => f.write_fmt(format_args!("{}", i)),
            Integer::Big(b) => f.write_fmt(format_args!("{}", *b)),
        }
    }
}

impl From<i64> for Integer {
    fn from(n: i64) -> Self { Integer::Small(n) }
}

impl From<u32> for Integer {
    fn from(n: u32) -> Self { Integer::Small(n as i64) }
}

impl From<i32> for Integer {
    fn from(n: i32) -> Self { Integer::Small(n as i64) }
}

impl Mul<i64> for Integer {
    type Output = Integer;
    fn mul(self, i1: i64) -> <Self as Mul<i64>>::Output {
        match self {
            Integer::Small(i0) =>
                if let Some(r) = i0.checked_mul(i1) {
                    Integer::Small(r)
                } else {
                    let b0 : BigInt = i0.into();
                    Integer::Big(Box::new(b0 * i1))
                }
            Integer::Big(b) =>
                Integer::Big(Box::new(*b * i1))
        }
    }
}

impl Rem<&Integer> for &Integer {
    type Output = Integer;
    fn rem(self, b: &Integer) -> <Self as Rem<&Integer>>::Output {
        match (self, b) {
            (Integer::Small(a), Integer::Small(b)) =>
                Integer::Small(a % b),
            
            (Integer::Big(a), Integer::Small(b)) => {
                let r = &**a % b;
                if let Ok(r1) = (&r).try_into() {
                    Integer::Small(r1)
                } else {
                    Integer::Big(Box::new(r))
                }
            }

            (Integer::Big(a), Integer::Big(b)) => {
                let r = &**a % &**b;
                if let Ok(r1) = (&r).try_into() {
                    Integer::Small(r1)
                } else {
                    Integer::Big(Box::new(r))
                }
            }

            (Integer::Small(a), Integer::Big(_)) => {
                // We guarantee that we only use Big if Small is too
                // small. Hence:
                Integer::Small(*a)
            }
        }
    }
}

impl Div<&Integer> for &Integer {
    type Output = Integer;
    fn div(self, b: &Integer) -> <Self as Rem<&Integer>>::Output {
        match (self, b) {
            (Integer::Small(a), Integer::Small(b)) =>
                // XXX overflows   MAX/-1   also % above ?
                Integer::Small(*a / *b),
            
            (Integer::Big(a), Integer::Small(b)) => {
                let r = &**a / *b;
                if let Ok(r1) = (&r).try_into() {
                    Integer::Small(r1)
                } else {
                    Integer::Big(Box::new(r))
                }
            }

            (Integer::Big(a), Integer::Big(b)) => {
                let r = &**a / &**b;
                if let Ok(r1) = (&r).try_into() {
                    Integer::Small(r1)
                } else {
                    Integer::Big(Box::new(r))
                }
            }

            (Integer::Small(_), Integer::Big(_)) => {
                // We guarantee that we only use Big if Small is too
                // small. Hence:
                Integer::Small(0)
            }
        }
    }
}

impl Add<i64> for Integer {
    type Output = Integer;
    fn add(self, i1: i64) -> <Self as Add<i64>>::Output {
        match self {
            Integer::Small(i0) =>
                if let Some(r) = i0.checked_add(i1) {
                    Integer::Small(r)
                } else {
                    let b0 : BigInt = i0.into();
                    Integer::Big(Box::new(b0 + i1))
                }
            Integer::Big(b) =>
                Integer::Big(Box::new(*b + i1))
        }
    }
}

impl Add<u32> for Integer {
    type Output = Integer;
    fn add(self, i1: u32) -> <Self as Add<u32>>::Output {
        self.add(i1 as i64)
    }
}

impl Neg for Integer {
    type Output = Integer;
    fn neg(self) -> <Self as Neg>::Output {
        match self {
            Integer::Small(i0) =>
                if let Some(r) = i0.checked_neg() {
                    Integer::Small(r)
                } else {
                    let b0 : BigInt = i0.into();
                    Integer::Big(Box::new(-b0))
                }
            Integer::Big(b) =>
                Integer::Big(Box::new(-*b))
        }
    }
}

impl Neg for &Integer {
    type Output = Integer;
    fn neg(self) -> <Self as Neg>::Output {
        match self {
            Integer::Small(i0) =>
                if let Some(r) = i0.checked_neg() {
                    Integer::Small(r)
                } else {
                    let b0 : BigInt = (*i0).into();
                    Integer::Big(Box::new(-b0))
                }
            Integer::Big(b) =>
                Integer::Big(Box::new(- (**b).clone()))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rational(pub Integer, pub Integer);

impl Neg for Rational {
    type Output = Rational;
    fn neg(self) -> <Self as Neg>::Output {
        Rational(self.0.neg(), self.1)
    }
}

fn abs(x: &Integer) -> Integer {
    if x < &0.into() {
        -x
    } else {
        x.clone()
    }
}

fn euclid(a: &Integer, b: &Integer) -> Integer {
    if b == &0.into() {
        a.clone()
    } else {
        let r = a % b;
        euclid(b, &r)
    }
}

fn gcd_positive(a: &Integer, b: &Integer) -> Integer {
    if a < b {
        euclid(b, a)
    } else {
        euclid(a, b)
    }
}

fn gcd(a: &Integer, b: &Integer) -> Integer {
    gcd_positive(&abs(a), &abs(b))
}

impl Rational {
    pub fn new(n: Integer, d: Integer) -> Rational {
        let f = gcd(&n, &d);
        if &f == &1.into() {
            Rational(n, d)
        } else {
            Rational(&n / &f, &d / &f)
        }
    }
}

/// TODO: complex numbers, inexact reals
#[derive(Debug, Clone, PartialEq)]
pub enum R5RSNumber {
    // Complex(Box<R5RSNumber>, Box<R5RSNumber>),
    // Real(f64),
    Rational(Box<Rational>),
    Integer(Integer)
}

impl std::fmt::Display for R5RSNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            R5RSNumber::Rational(n) =>
                f.write_fmt(format_args!("{}/{}", n.0, n.1)),
            R5RSNumber::Integer(n) => f.write_fmt(format_args!("{}", n)),
        }
    }
}

