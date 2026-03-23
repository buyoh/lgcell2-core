use crate::circuit::{Pos, Wire, WireKind};

#[test]
fn propagate_positive_keeps_value() {
    let wire = Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive);
    assert!(wire.propagate(true));
    assert!(!wire.propagate(false));
}

#[test]
fn propagate_negative_inverts_value() {
    let wire = Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Negative);
    assert!(!wire.propagate(true));
    assert!(wire.propagate(false));
}
