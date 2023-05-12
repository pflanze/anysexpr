
use anysexpr::pos::Pos;
use anysexpr::value::{VValue, Parenkind};
use anysexpr::read::read_file;
use anysexpr::parse::{Token, ParseSettings, parse, TokenWithPos};
use anysexpr::buffered_chars::buffered_chars;
use clap::Parser as ClapParser;
use std::path::PathBuf;
use anyhow::{Result, bail};


fn indentstr(i: usize) -> Option<&'static str> {
    let range = 0..i;
    "                                                                  ".get(range)
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

        // Slurp in the whole file contents as a tree
        let v: Vec<VValue> = read_file(&args.input_path)?;
        if args.print {
            let len = v.len();
            for (i, item) in v.iter().enumerate() {
                println!("{}{}",
                         item,
                         if i + 1 < len { "\n" } else { "" });
            }
        }

    } else {

        // Read through the token stream of the file contents and just
        // do some bookkeeping and optionally print the tokens
        let fh = std::fs::File::open(&args.input_path)?;
        let mut cs = buffered_chars(fh);
        let ts = parse(&mut cs,
                       ParseSettings {
                           whitespace: args.whitespace,
                           comments: args.comments,
                       });
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
            if args.print {
                if let Some(indent) = indentstr(indentlevel) {
                    if args.pos {
                        println!("{indent}{pos} {token}");
                    } else {
                        println!("{indent}{token}");
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