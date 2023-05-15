# Any-sexpr

This is an S-Expression parser and formatter with the following features:

* Offering direct access to the tokenizer, `anysexpr::parse`, but also
  `anysexpr::read` to build an in-memory tree easily.

* Good error reporting (precise location information and
  messages).

* Future: make the data constructors for `anysexpr::read`
  parametrizable (generic), e.g. like in the `sexpr_parser` crate.

* Streaming: allow to read from and print to file handles lazily, for
  use e.g. in communications. This currently works by using
  `anysexpr::parse` directly for input, or creating tokens to print
  via a custom loop for output. Future: more possibilities, e.g. turn
  a tree into a token stream, or parameterize with a tree that's
  generated on demand while printing.

* Future: support various s-expression variants (R*RS, Guile, Clojure,
  Common Lisp, ..) via runtime (and compile-time?) settings.

This is an early alpha version. Even some basics are unfinished
(inexact and complex numbers, quoting sugar, some comment styles, ..).
The author is quite new to Rust. He is aware of the huge list of API
guideline entries not currently being followed, and welcomes help in
that area as well as others. The author is also quite invested in
lisps and expects to support this library for a long time.

## Todo

* string escape features on printing
* inexact and complex numbers
* chars: `#\x...`, `#\u...`, `#\U...`
* implement quote/quasiquote/unquote parsing
* handle `#| |#` and `#;` style comments
* Guile style keywords `#:foo`
* better errors, do not use anyhow?
* change `path` context into a container, embed it in the
  errors, replacing `locator`
* performance tuning?, perhaps do not use genawaiter?
* better error behaviour: parser should return errors but try to be
  able to continue?
* tests (large test corpora, round trip tests, fuzzing)
* handle Guile, Clojure and other syntax versions
* parametrization (generics) for tree generation / mapping (also/vs. Serde?)
* lazy features as mentioned above
* some level of support for pretty-printing

## Contact

Christian Jaeger <ch@christianjaeger.ch>

