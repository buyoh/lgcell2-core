use std::collections::BTreeSet;

use crate::circuit::{Circuit, Generator, Input, Output, Pos, ResolvedModule, Tester, Wire, WireKind};

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
    let inputs = vec![Input::Generator(Generator::new(
        Pos::new(9, 9),
        vec![true],
        false,
    ))];

    let circuit =
        Circuit::with_components(cells, wires, inputs, Vec::new()).expect("circuit must be valid");

    assert!(circuit.cells().contains(&Pos::new(9, 9)));
    assert_eq!(circuit.inputs().len(), 1);
}

#[test]
fn circuit_rejects_generator_target_with_incoming_wire() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(2, 0),
        WireKind::Positive,
    )];
    let inputs = vec![Input::Generator(Generator::new(
        Pos::new(2, 0),
        vec![true],
        false,
    ))];

    let err = Circuit::with_components(cells, wires, inputs, Vec::new())
        .expect_err("must reject input on incoming target");
    assert!(matches!(
        err,
        crate::base::CircuitError::InputTargetHasIncomingWires(Pos { x: 2, y: 0 })
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
    let inputs = vec![
        Input::Generator(Generator::new(Pos::new(2, 0), vec![true], false)),
        Input::Generator(Generator::new(Pos::new(2, 0), vec![false], true)),
    ];

    let err = Circuit::with_components(cells, wires, inputs, Vec::new())
        .expect_err("must reject duplicate input target");
    assert!(matches!(
        err,
        crate::base::CircuitError::DuplicateInputTarget(Pos { x: 2, y: 0 })
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
    let inputs = vec![Input::Generator(Generator::new(
        Pos::new(2, 0),
        Vec::new(),
        false,
    ))];

    let err = Circuit::with_components(cells, wires, inputs, Vec::new())
        .expect_err("must reject empty generator pattern");
    assert!(matches!(
        err,
        crate::base::CircuitError::EmptyGeneratorPattern(Pos { x: 2, y: 0 })
    ));
}

#[test]
fn circuit_rejects_duplicate_output_target() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];
    let outputs = vec![
        Output::Tester(Tester::new(Pos::new(1, 0), vec![Some(true)], false)),
        Output::Tester(Tester::new(Pos::new(1, 0), vec![Some(false)], false)),
    ];

    let err = Circuit::with_components(cells, wires, Vec::new(), outputs)
        .expect_err("must reject duplicate output target");
    assert!(matches!(
        err,
        crate::base::CircuitError::DuplicateOutputTarget(Pos { x: 1, y: 0 })
    ));
}

#[test]
fn circuit_rejects_empty_tester_pattern() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];
    let outputs = vec![Output::Tester(Tester::new(Pos::new(1, 0), Vec::new(), false))];

    let err = Circuit::with_components(cells, wires, Vec::new(), outputs)
        .expect_err("must reject empty tester pattern");
    assert!(matches!(
        err,
        crate::base::CircuitError::EmptyTesterPattern(Pos { x: 1, y: 0 })
    ));
}

// --- with_modules tests ---

fn sample_sub_circuit() -> Circuit {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0)]);
    let wires = vec![Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Negative)];
    Circuit::new(cells, wires).expect("sub-circuit must be valid")
}

fn sample_module() -> ResolvedModule {
    ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(1, 0)],
        vec![Pos::new(2, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    )
}

#[test]
fn circuit_with_modules_empty_modules() {
    let cells = sample_cells();
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];

    let circuit = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), Vec::new())
        .expect("circuit must be valid");
    assert!(circuit.modules().is_empty());
}

#[test]
fn circuit_with_modules_valid() {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0)]);
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Negative,
    )];
    let module = sample_module();

    let circuit = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module])
        .expect("circuit must be valid");
    assert_eq!(circuit.modules().len(), 1);
    assert!(circuit.cells().contains(&Pos::new(2, 0)));
}

#[test]
fn circuit_with_modules_rejects_output_with_incoming_wire() {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)]);
    let wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Negative),
        Wire::new(Pos::new(1, 0), Pos::new(2, 0), WireKind::Positive),
    ];
    let module = sample_module();

    let err = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module])
        .expect_err("must reject output with incoming wire");
    assert!(matches!(
        err,
        crate::base::CircuitError::ModuleOutputHasIncomingWires(Pos { x: 2, y: 0 })
    ));
}

#[test]
fn circuit_with_modules_rejects_duplicate_output() {
    let cells = BTreeSet::from([
        Pos::new(0, 0),
        Pos::new(1, 0),
        Pos::new(3, 0),
        Pos::new(4, 0),
    ]);
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Negative,
    )];
    let module1 = ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(3, 0)],
        vec![Pos::new(4, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );
    let module2 = ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(3, 0)],
        vec![Pos::new(4, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );

    let err = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module1, module2])
        .expect_err("must reject duplicate module output");
    assert!(matches!(
        err,
        crate::base::CircuitError::DuplicateModuleOutput(Pos { x: 4, y: 0 })
    ));
}

#[test]
fn circuit_with_modules_rejects_output_before_input() {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0)]);
    let wires = vec![];
    // output x=1 <= input x=2 → error
    let module = ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(2, 0)],
        vec![Pos::new(1, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );

    let err = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module])
        .expect_err("must reject output before input");
    assert!(matches!(
        err,
        crate::base::CircuitError::ModuleOutputBeforeInput
    ));
}

#[test]
fn circuit_with_modules_rejects_invalid_port_column_different_x() {
    let cells = BTreeSet::from([Pos::new(0, 0)]);
    let wires = vec![];
    // input ports with different x → error
    let module = ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(1, 0), Pos::new(2, 1)],
        vec![Pos::new(3, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );

    let err = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module])
        .expect_err("must reject invalid port column");
    assert!(matches!(
        err,
        crate::base::CircuitError::InvalidPortColumn
    ));
}

#[test]
fn circuit_with_modules_rejects_invalid_port_column_non_contiguous_y() {
    let cells = BTreeSet::from([Pos::new(0, 0)]);
    let wires = vec![];
    // input ports same x but y not contiguous (0, 2) → error
    let module = ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(1, 0), Pos::new(1, 2)],
        vec![Pos::new(3, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );

    let err = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module])
        .expect_err("must reject non-contiguous y");
    assert!(matches!(
        err,
        crate::base::CircuitError::InvalidPortColumn
    ));
}

#[test]
fn circuit_with_modules_rejects_output_on_generator_target() {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0)]);
    let wires = vec![];
    let inputs = vec![Input::Generator(Generator::new(
        Pos::new(2, 0),
        vec![true],
        false,
    ))];
    // module output at generator target (2,0) → error
    let module = ResolvedModule::new(
        sample_sub_circuit(),
        vec![Pos::new(1, 0)],
        vec![Pos::new(2, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );

    let err = Circuit::with_modules(cells, wires, inputs, Vec::new(), vec![module])
        .expect_err("must reject output on generator target");
    assert!(matches!(
        err,
        crate::base::CircuitError::ModuleOutputHasIncomingWires(Pos { x: 2, y: 0 })
    ));
}

#[test]
fn circuit_with_modules_multiple_ports_valid() {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(0, 1)]);
    let wires = vec![];
    let sub_cells = BTreeSet::from([
        Pos::new(0, 0),
        Pos::new(0, 1),
        Pos::new(1, 0),
        Pos::new(1, 1),
    ]);
    let sub_wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Negative),
        Wire::new(Pos::new(0, 1), Pos::new(1, 1), WireKind::Negative),
    ];
    let sub_circuit = Circuit::new(sub_cells, sub_wires).expect("sub-circuit must be valid");

    let module = ResolvedModule::new(
        sub_circuit,
        vec![Pos::new(1, 0), Pos::new(1, 1)],
        vec![Pos::new(2, 0), Pos::new(2, 1)],
        vec![Pos::new(0, 0), Pos::new(0, 1)],
        vec![Pos::new(1, 0), Pos::new(1, 1)],
    );

    let circuit = Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), vec![module])
        .expect("circuit must be valid");
    assert_eq!(circuit.modules().len(), 1);
    assert!(circuit.cells().contains(&Pos::new(2, 0)));
    assert!(circuit.cells().contains(&Pos::new(2, 1)));
}
