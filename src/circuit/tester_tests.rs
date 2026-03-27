use crate::circuit::{Pos, Tester};

#[test]
fn expected_at_non_loop_returns_none_after_pattern_end() {
    let tester = Tester::new(Pos::new(1, 0), vec![Some(true), Some(false)], false);

    assert_eq!(tester.expected_at(0), Some(true));
    assert_eq!(tester.expected_at(1), Some(false));
    assert_eq!(tester.expected_at(2), None);
}

#[test]
fn expected_at_loop_repeats_pattern() {
    let tester = Tester::new(
        Pos::new(1, 0),
        vec![Some(true), None, Some(false)],
        true,
    );

    assert_eq!(tester.expected_at(0), Some(true));
    assert_eq!(tester.expected_at(1), None);
    assert_eq!(tester.expected_at(2), Some(false));
    assert_eq!(tester.expected_at(3), Some(true));
}