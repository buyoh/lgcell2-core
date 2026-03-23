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
