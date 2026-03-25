use std::collections::BTreeSet;

use crate::circuit::{Circuit, Generator, Pos, Wire, WireKind};

fn sample_cells() -> BTreeSet<Pos> {
    BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)])
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
    assert!(matches!(
        err,
        crate::base::CircuitError::WireSrcNotFound(Pos { x: 9, y: 9 })
    ));
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
    assert!(matches!(
        err,
        crate::base::CircuitError::SelfLoop {
            src: Pos { x: 1, y: 0 },
            dst: Pos { x: 1, y: 0 }
        }
    ));
}

#[test]
fn circuit_rejects_duplicate_wire_same_kind() {
    let cells = sample_cells();
    let wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Positive),
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Positive),
    ];

    let err = Circuit::new(cells, wires).expect_err("must reject duplicate wire");
    assert!(matches!(
        err,
        crate::base::CircuitError::DuplicateWire {
            src: Pos { x: 0, y: 0 },
            dst: Pos { x: 2, y: 0 }
        }
    ));
}

#[test]
fn circuit_rejects_duplicate_wire_different_kind() {
    let cells = sample_cells();
    let wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Positive),
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Negative),
    ];

    let err = Circuit::new(cells, wires).expect_err("must reject duplicate wire");
    assert!(matches!(
        err,
        crate::base::CircuitError::DuplicateWire {
            src: Pos { x: 0, y: 0 },
            dst: Pos { x: 2, y: 0 }
        }
    ));
}

#[test]
fn circuit_allows_reverse_direction_wires() {
    let cells = sample_cells();
    let wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Positive),
        Wire::new(Pos::new(2, 0), Pos::new(0, 0), WireKind::Positive),
    ];

    let circuit = Circuit::new(cells, wires).expect("circuit must be valid");
    assert_eq!(circuit.wires().len(), 2);
}

#[test]
fn circuit_with_generators_adds_targets_to_cells() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];
    let generators = vec![Generator::new(Pos::new(9, 9), vec![true], false)];

    let circuit =
        Circuit::with_generators(cells, wires, generators).expect("circuit must be valid");

    assert!(circuit.cells().contains(&Pos::new(9, 9)));
    assert_eq!(circuit.generators().len(), 1);
}

#[test]
fn circuit_rejects_generator_target_with_incoming_wire() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(2, 0),
        WireKind::Positive,
    )];
    let generators = vec![Generator::new(Pos::new(2, 0), vec![true], false)];

    let err = Circuit::with_generators(cells, wires, generators)
        .expect_err("must reject generator on incoming target");
    assert!(matches!(
        err,
        crate::base::CircuitError::GeneratorTargetHasIncomingWires(Pos { x: 2, y: 0 })
    ));
}

#[test]
fn circuit_rejects_duplicate_generator_target() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];
    let generators = vec![
        Generator::new(Pos::new(2, 0), vec![true], false),
        Generator::new(Pos::new(2, 0), vec![false], true),
    ];

    let err = Circuit::with_generators(cells, wires, generators)
        .expect_err("must reject duplicate generator target");
    assert!(matches!(
        err,
        crate::base::CircuitError::DuplicateGeneratorTarget(Pos { x: 2, y: 0 })
    ));
}

#[test]
fn circuit_rejects_empty_generator_pattern() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];
    let generators = vec![Generator::new(Pos::new(2, 0), Vec::new(), false)];

    let err = Circuit::with_generators(cells, wires, generators)
        .expect_err("must reject empty generator pattern");
    assert!(matches!(
        err,
        crate::base::CircuitError::EmptyGeneratorPattern(Pos { x: 2, y: 0 })
    ));
}
