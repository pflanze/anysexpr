// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::pos::Pos;
use std::{fs, io};
use anyhow::{Result, anyhow};
use utf8::BufReadDecoder;
use genawaiter::rc::Gen;


pub fn buffered_chars(fh: fs::File)
                      -> impl Iterator<Item=Result<(char, Pos)>>
{
    Gen::new(|co| async move {
        let mut inp = BufReadDecoder::new(io::BufReader::new(fh));
        let mut pos = Pos { line: 0, col: 0 };
        loop {
            if let Some(r) = inp.next_strict() {
                match r {
                    Ok(x) => {
                        for c in x.chars() {
                            co.yield_(Ok((c, pos))).await;
                            pos =
                                if c == '\n' {
                                    Pos { line: pos.line + 1, col: 0 }
                                } else {
                                    Pos { line: pos.line, col: pos.col + 1 }
                                };
                        }
                    },
                    Err(e) => {
                        co.yield_(Err(anyhow!("buffered_chars: {}", e))).await;
                        return;
                    }
                }
            } else {
                return;
            }
        }
    }).into_iter()
}

