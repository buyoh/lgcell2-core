use std::collections::BTreeSet;

use crate::circuit::{Circuit, Pos, Wire, WireKind};

fn sample_cells() -> BTreeSet<Pos> {
    BTreeSet::from([
        Pos::new(0, 0),
        Pos::new(1, 0),
        Pos::new(2, 0),
    ])
}

#[test]
fn circuit_builds_incoming_index() {
    let cells = sample_cells();
    let wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Positive),
        Wire::new(Pos::new(1, 0), Pos::new(2, 0), WireKind::Negative),
    ];

    let circuit = Circuit::new(cells, wires).expect("circuit must be valid");

    assert_eq!(circuit.incoming_indices(Pos::new(2, 0)), &[0, 1]);
    assert!(circuit.incoming_indices(Pos::new(0, 0)).is_empty());
}

#[test]
fn circuit_keeps_sorted_cells() {
    let cells = BTreeSet::from([Pos::new(1, 1), Pos::new(0, 3), Pos::new(1, -1)]);

    let circuit = Circuit::new(cells, Vec::new()).expect("circuit must be valid");

    assert_eq!(
        circuit.sorted_cells(),
        &[Pos::new(0, 3), Pos::new(1, -1), Pos::new(1, 1)]
    );
}

#[test]
fn circuit_rejects_unknown_wire_endpoint() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(9, 9),
        Pos::new(2, 0),
        WireKind::Positive,
    )];

    let err = Circuit::new(cells, wires).expect_err("must reject unknown src");
    assert!(err.contains("wire src does not exist"));
}

#[test]
fn circuit_rejects_self_loop() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(1, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];

    let err = Circuit::new(cells, wires).expect_err("must reject self-loop");
    assert!(err.contains("self-loop wire is not allowed"));
}
