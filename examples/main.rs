// Copyright 2023 Christian Jaeger <ch@christianjaeger.ch>. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anysexpr::pos::Pos;
use anysexpr::value::{Parenkind, VValueWithPos};
use anysexpr::read::{read_file, write_all};
use anysexpr::parse::{Token, parse, TokenWithPos};
use anysexpr::settings::{Settings, Modes, GAMBIT_FORMAT};
use anysexpr::buffered_chars::buffered_chars;
use clap::Parser as ClapParser;
use std::io::{stdout, BufWriter, Write};
use std::path::PathBuf;
use anyhow::{Result, bail};


fn indentstr(i: usize) -> Option<&'static str> {
    "                                                                  ".get(0..i)
}

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Build up a tree of all content (default: stream tokens)
    #[clap(short, long, value_parser)]
    ast: bool,
    /// Print the parsed data
    #[clap(long, value_parser)]
    print: bool,
    /// Write a debugging dump of the parsed data (with --ast, as
    /// s-expression, without --ast, using Debug instead of Display)
    #[clap(long, value_parser)]
    dump: bool,
    /// Show the token position (only with --print and no --ast)
    #[clap(long, value_parser)]
    pos: bool,
    /// Show the whitespace (only with --print and no --ast)
    #[clap(short, long, value_parser)]
    whitespace: bool,
    /// Show the comments (only with --print and no --ast)
    #[clap(short, long, value_parser)]
    comments: bool,
    /// Path to the input file
    #[clap(value_parser, required(true))]
    input_path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.ast {

        // Slurp in the whole file contents as a list of trees, then
        // optionally print those.
        
        let vals: Vec<VValueWithPos> = read_file(&args.input_path)?;
        if args.print {
            write_all(BufWriter::new(stdout()), &vals)?;
        }
        if args.dump {
            let vals2 : Vec<VValueWithPos> = vals.iter().map(|v| v.dump()).collect();
            write_all(BufWriter::new(stdout()), &vals2)?;
        }

    } else {

        // Read through the token stream of the file contents and just
        // do some bookkeeping and optionally print the tokens.

        let mut out = BufWriter::new(stdout());
        let fh = std::fs::File::open(&args.input_path)?;
        let mut cs = buffered_chars(fh);
        let settings = Settings {
            format: &GAMBIT_FORMAT,
            modes: &Modes {
                retain_whitespace: args.whitespace,
                retain_comments: args.comments,
            }};
        let ts = parse(&mut cs, &settings);
        let mut count_toplevel = 0;
        let mut count_enter = 0;
        let mut parenstack: Vec<(Parenkind, Pos)> = Vec::new();
        for te in ts {
            let TokenWithPos(token, pos) = te?;
            let indentlevel;
            match token {
                Token::Open(kind) => {
                    count_enter += 1;
                    if parenstack.is_empty() {
                        count_toplevel += 1;
                    }
                    indentlevel = parenstack.len();
                    parenstack.push((kind, pos));
                }
                Token::Close(kind) => {
                    if let Some((expected_kind, opening_pos)) = parenstack.pop() {
                        if kind != expected_kind {
                            bail!("expected closing character '{}' (opening {}), \
                                   got '{}' at {:?}{}",
                                  expected_kind.closing(),
                                  opening_pos,
                                  kind.closing(),
                                  args.input_path,
                                  pos)
                        }
                        indentlevel = parenstack.len();
                    } else {
                        bail!("unexpected closing character '{}' at {:?}{}",
                              kind.closing(), args.input_path, pos)
                    }
                }
                _ => {
                    indentlevel = parenstack.len();
                }
            }
            if args.print || args.dump {
                if let Some(indent) = indentstr(indentlevel) {
                    out.write_all(indent.as_bytes())?;
                    if args.pos {
                        write!(out, "{pos} ")?;
                    }
                    if args.dump {
                        write!(out, "{token:?}\n")?;
                    } else {
                        write!(out, "{token}\n")?;
                    }
                } else {
                    bail!("lists nested too deeply at {:?}{}", args.input_path, pos)
                }
            }
        }
        println!(";; count_toplevel = {count_toplevel}, count_enter = {count_enter}");

    }
    Ok(())
}
