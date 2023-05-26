// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use std::io::Write;
use anysexpr::{read::{read_all, write_all, writeln}, value::VValueWithPos};

const INPUT: &[u8] = include_bytes!("t-input.scm");
const WRITE: &[u8] = include_bytes!("t-write.scm");
const DUMP: &[u8] = include_bytes!("t-dump.scm");

#[test]
fn roundtrip1() -> Result<()> {
    let vals = read_all(INPUT)?;
    let mut out = Vec::<u8>::new();
    write_all(&mut out, &vals)?;
    assert_eq!(out, WRITE);
    Ok(())
}

#[test]
fn dump() -> Result<()> {
    let vals : Vec<VValueWithPos> = read_all(INPUT)?.iter().map(|v| v.dump()).collect();
    let mut out = Vec::<u8>::new();
    // Copy from examples/main.rs, keep in sync!
    for val in vals {
        // Print line information as s-expression
        write!(&mut out, "(line {})\n", val.1.line + 1)?;
        writeln(&mut out, &val.dump())?;
    }
    assert_eq!(out, DUMP);
    Ok(())
}
