use std::collections::BTreeSet;

use crate::circuit::{Circuit, Pos, Wire, WireKind};
use crate::simulation::{Simulator, StepResult};

fn make_circuit(cells: &[Pos], wires: Vec<Wire>) -> Circuit {
    Circuit::new(BTreeSet::from_iter(cells.iter().copied()), wires).expect("valid circuit")
}

#[test]
fn positive_chain_propagates_within_one_tick() {
    let circuit = make_circuit(
        &[
            Pos::new(0, 0),
            Pos::new(1, 0),
            Pos::new(2, 0),
        ],
        vec![
            Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive),
            Wire::new(Pos::new(1, 0), Pos::new(2, 0), WireKind::Positive),
        ],
    );

    let mut sim = Simulator::new(circuit);
    sim.tick();

    // 初期値が全て 0 のため、Positive 伝搬しても全て false のまま
    assert_eq!(sim.state().get(Pos::new(1, 0)), Some(false));
    assert_eq!(sim.state().get(Pos::new(2, 0)), Some(false));
}

#[test]
fn backward_wire_is_delayed_by_one_tick() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(1, 0),
            Pos::new(0, 0),
            WireKind::Positive,
        )],
    );

    let mut sim = Simulator::new(circuit);
    sim.tick();
    // 初期値が全て 0 のため、Positive 伝搬しても false のまま
    assert_eq!(sim.state().get(Pos::new(0, 0)), Some(false));
}

#[test]
fn nand_is_constructed_by_two_negative_wires() {
    let circuit = make_circuit(
        &[
            Pos::new(0, 0),
            Pos::new(1, 0),
            Pos::new(2, 0),
        ],
        vec![
            Wire::new(Pos::new(0, 0), Pos::new(2, 0), WireKind::Negative),
            Wire::new(Pos::new(1, 0), Pos::new(2, 0), WireKind::Negative),
        ],
    );

    let mut sim = Simulator::new(circuit);
    sim.tick();
    // 入力が両方 false → Negative で反転 → 両方 true → OR = true
    assert_eq!(sim.state().get(Pos::new(2, 0)), Some(true));
}

#[test]
fn step_can_pause_and_resume_without_behavior_change() {
    let circuit = make_circuit(
        &[
            Pos::new(0, 0),
            Pos::new(1, 0),
            Pos::new(2, 0),
        ],
        vec![
            Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive),
            Wire::new(Pos::new(1, 0), Pos::new(2, 0), WireKind::Positive),
        ],
    );

    let mut by_tick = Simulator::new(circuit.clone());
    by_tick.tick();

    let mut by_step = Simulator::new(circuit);
    assert_eq!(by_step.step(), StepResult::Continue);
    assert_eq!(by_step.current_tick(), 0);
    assert_eq!(by_step.step(), StepResult::Continue);
    assert_eq!(by_step.step(), StepResult::TickComplete);

    assert_eq!(by_tick.state(), by_step.state());
}
