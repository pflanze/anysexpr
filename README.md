# Any-sexpr

This is an S-Expression parser and formatter with the following features:

* Offering direct access to the tokenizer, `anysexpr::parse`, but also
  `anysexpr::read` to build an in-memory tree easily.

* Future: make the data constructors for `anysexpr::read`
  parametrizable (generic), e.g. like in the `sexpr_parser` crate.

* Streaming: allow to read from and print to file handles lazily. This
  currently works by using `anysexpr::parse` directly, or creating
  tokens to print via a custom loop. Future: more possibilities,
  e.g. turn a tree into a token stream, or parameterize with a tree
  that's generated on demand while printing.

* Future: support various s-expression versions (R5RS, Guile, Clojure,
  Common Lisp, ..) via runtime (and compile-time?) settings.

This is an early alpha version. Even some basics are unfinished
(non-integer numbers, quoting sugar, some comment styles, ..).

The author is quite new in Rust but quite invested in lisps, this
will evolve.

## Todo

* string escape features on printing
* numbers other than integers
* chars
* `#f`, `#t`
* implement quote/quasiquote/unquote parsing
* handle `#| |#` and `#;` style comments
* Guile style keywords
* better errors, do not use anyhow?
* change `path` context into a container, embedded it in the
  errors, replacing `locator`
* performance tuning?, perhaps do not use genawaiter?
* better error behaviour: parser should return errors but try to be
  able to continue?
* tests
* handle Guile, Clojure and other syntax versions
* parametrization (generics) for tree generation
* lazy features as mentioned above
* some level of support for pretty-printing

## Contact

Christian Jaeger <ch@christianjaeger.ch>

