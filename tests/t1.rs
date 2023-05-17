use anysexpr::read::{read_all, write_all};

const INPUT: &[u8] = include_bytes!("t-input.scm");
const EXPECTED: &[u8] = include_bytes!("t-expected.scm");

#[test]
fn t1() {
    let vals = read_all(INPUT).unwrap();
    let mut out = Vec::<u8>::new();
    write_all(&mut out, &vals).unwrap();
    assert_eq!(out, EXPECTED);
}
