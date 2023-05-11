use std::cmp::Eq;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        // This, when prefixed with a Debug style path string, is
        // following the Emacs convention for location information.
        f.write_fmt(format_args!("@{}.{}", self.line + 1, self.col))
    }
}

