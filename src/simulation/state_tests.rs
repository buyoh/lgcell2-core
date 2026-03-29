use std::collections::BTreeSet;

use crate::circuit::{Circuit, Generator, Input, Pos, Wire, WireKind};
use crate::simulation::WireSimState;

fn build_circuit() -> Circuit {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)]);
    let wires = vec![
        Wire::new(Pos::new(1, 0), Pos::new(0, 0), WireKind::Positive),
        Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Positive),
    ];
    Circuit::new(cells, wires).expect("valid circuit")
}

#[test]
fn delayed_wire_slot_is_created_for_backward_wire() {
    let circuit = build_circuit();
    let state = WireSimState::from_circuit(&circuit);

    assert_eq!(state.get_delayed_wire(0), Some(false));
    assert_eq!(state.get_delayed_wire(1), None);
}

#[test]
fn stateless_cell_slot_is_created_for_inputless_cell() {
    let circuit = build_circuit();
    let state = WireSimState::from_circuit(&circuit);

    // cell index 1 (1,0) は入力なしセル
    assert_eq!(state.get_stateless_cell(1), Some(false));
    assert_eq!(state.get_stateless_cell(0), None);
}

#[test]
fn input_target_is_excluded_from_stateless_slot() {
    let cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0)]);
    let inputs = vec![Input::Generator(Generator::new(
        Pos::new(0, 0),
        vec![true],
        false,
    ))];
    let circuit = Circuit::with_components(cells, Vec::new(), inputs, Vec::new())
        .expect("valid circuit");

    let state = WireSimState::from_circuit(&circuit);
    assert_eq!(state.get_stateless_cell(0), None);
    assert_eq!(state.get_stateless_cell(1), Some(false));
}

#[test]
fn update_methods_change_slot_values() {
    let circuit = build_circuit();
    let mut state = WireSimState::from_circuit(&circuit);

    state.update_wire(0, true);
    state.update_cell(1, true);

    assert_eq!(state.get_delayed_wire(0), Some(true));
    assert_eq!(state.get_stateless_cell(1), Some(true));
}
