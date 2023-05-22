// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use anysexpr::read::{read_all, write_all};

const INPUT: &[u8] = include_bytes!("t-input.scm");
const EXPECTED: &[u8] = include_bytes!("t-expected.scm");

#[test]
fn t1() -> Result<()> {
    let vals = read_all(INPUT)?;
    let mut out = Vec::<u8>::new();
    write_all(&mut out, &vals)?;
    assert_eq!(out, EXPECTED);
    Ok(())
}
