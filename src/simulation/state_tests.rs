use std::collections::BTreeMap;

use crate::circuit::{Circuit, Pos};
use crate::simulation::SimState;

fn build_circuit() -> Circuit {
    let cells = BTreeMap::from([(Pos::new(0, 0), false), (Pos::new(1, 0), true)]);
    Circuit::new(cells, Vec::new()).expect("valid circuit")
}

#[test]
fn state_is_initialized_from_circuit_cells() {
    let circuit = build_circuit();
    let state = SimState::from_circuit(&circuit);

    assert_eq!(state.get(Pos::new(0, 0)), Some(false));
    assert_eq!(state.get(Pos::new(1, 0)), Some(true));
}

#[test]
fn set_updates_existing_cell() {
    let circuit = build_circuit();
    let mut state = SimState::from_circuit(&circuit);

    state
        .set(Pos::new(0, 0), true)
        .expect("existing position must update");

    assert_eq!(state.get(Pos::new(0, 0)), Some(true));
}

#[test]
fn set_rejects_unknown_cell() {
    let circuit = build_circuit();
    let mut state = SimState::from_circuit(&circuit);

    let err = state
        .set(Pos::new(9, 9), true)
        .expect_err("unknown position must fail");

    assert!(err.contains("unknown cell"));
}
