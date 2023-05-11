use num::BigInt;

#[derive(Debug)]
pub enum R5RSNumber {
    // Complex(Box<R5RSNumber>, Box<R5RSNumber>),
    // Real(f64),
    // Rational(Box<BigInt>, Box<BigInt>),
    // ^ boxing since BigInt is Vec (RawVec (ptr and usize) and usize)
    //   plus sign. Proper optimization later, though.
    Integer(BigInt)
}

impl std::fmt::Display for R5RSNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
           -> Result<(), std::fmt::Error> {
        match self {
            R5RSNumber::Integer(n) => f.write_fmt(format_args!("{}", n)),
        }
    }
}

