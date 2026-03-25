use super::Pos;

#[test]
fn pos_ord_is_lexicographic_by_x_then_y() {
    let mut positions = [Pos::new(1, 0), Pos::new(0, 2), Pos::new(0, 1), Pos::new(1, -1)];
    positions.sort();

    assert_eq!(
        positions,
        [Pos::new(0, 1), Pos::new(0, 2), Pos::new(1, -1), Pos::new(1, 0)]
    );
}

#[test]
fn display_formats_as_parenthesized_pair() {
    assert_eq!(Pos::new(0, 0).to_string(), "(0, 0)");
    assert_eq!(Pos::new(3, -1).to_string(), "(3, -1)");
}
