# Any-sexpr

This is an S-Expression parser and formatter with the following goals:

* Offering direct access to the tokenizer, `anysexpr::parse`, but also
  `anysexpr::read` to build an in-memory tree easily.

* Good error reporting (precise location information and
  messages).

* (Future) Make the data constructors for `anysexpr::read`
  parametrizable (generic), e.g. like in the `sexpr_parser` crate.

* Streaming: allow to read from and print to file handles lazily, for
  use e.g. in communications. This currently works by using
  `anysexpr::parse` directly for input, or creating tokens to print
  via a custom loop for output. Future: more possibilities, e.g. turn
  a tree into a token stream, or parameterize with a tree that's
  generated on demand while printing.

* (Future) Support various s-expression variants (R*RS, Guile, Clojure,
  Common Lisp, ..) via runtime (and compile-time?) settings.

* (Perhaps) be usable on microcontrollers (small code, no-std?).

The author is quite new to Rust. There will be API guideline entries
not currently being followed, help in that area is as welcome as in
other areas.

## Usage

See [examples/main.rs](examples/main.rs).

## Todo

* better string printing: escape features
* better symbol printing: more properly detect whether delimiters are
  needed
* inexact and complex numbers
* performance tuning (perhaps do not use genawaiter? optimize error struct sizes.)
* intern the symbols ([value.rs](src/value.rs))
* better error behaviour: parser should return errors but try to make
  it possible to continue? Does that require passing the next token in
  the error and re-using it, or should parsing use Peekable?
* more tests (large test corpora, fuzzing round trips)
* handle Guile, Clojure and other syntax versions
* parametrization (generics) for tree generation / mapping (also/vs. Serde?)
* lazy features as mentioned above
* some level of support for pretty-printing

Orthogonally:

* examine the `lexpr` crate in more detail (missed when researching
  existing crates before starting this project)

## Contact

Christian Jaeger <ch@christianjaeger.ch>

