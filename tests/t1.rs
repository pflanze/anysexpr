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
use std::str;
use anysexpr::{buffered_chars::buffered_chars, settings::GAMBIT_FORMAT};

const INPUT: &[u8] = include_bytes!("t-input.scm");
const WRITE: &[u8] = include_bytes!("t-write.scm");
const DUMP: &[u8] = include_bytes!("t-dump.scm");

#[test]
fn roundtrip1() -> Result<()> {
    let vals = GAMBIT_FORMAT.read_all(buffered_chars(INPUT))?;
    let mut out = Vec::<u8>::new();
    GAMBIT_FORMAT.write_all(&mut out, &vals)?;
    assert_eq!(str::from_utf8(&out), str::from_utf8(WRITE));
    Ok(())
}

#[test]
fn dump() -> Result<()> {
    let vals = GAMBIT_FORMAT.read_all(buffered_chars(INPUT))?;
    let mut out = Vec::<u8>::new();
    // Copy from examples/main.rs, keep in sync!
    for val in vals {
        // Print line information as s-expression
        write!(&mut out, "(line {})\n", val.1.line + 1)?;
        GAMBIT_FORMAT.writeln(&mut out, &val.dump())?;
    }
    assert_eq!(str::from_utf8(&out), str::from_utf8(DUMP));
    Ok(())
}
