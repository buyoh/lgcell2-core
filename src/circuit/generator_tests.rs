use crate::circuit::{Generator, Pos};

#[test]
fn value_at_non_loop_holds_last_value() {
    let generator = Generator::new(Pos::new(0, 0), vec![true, false], false);

    assert!(generator.value_at(0));
    assert!(!generator.value_at(1));
    assert!(!generator.value_at(2));
    assert!(!generator.value_at(10));
}

#[test]
fn value_at_loop_repeats_pattern() {
    let generator = Generator::new(Pos::new(0, 0), vec![true, false], true);

    assert!(generator.value_at(0));
    assert!(!generator.value_at(1));
    assert!(generator.value_at(2));
    assert!(!generator.value_at(3));
}
