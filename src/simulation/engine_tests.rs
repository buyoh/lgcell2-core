use std::collections::BTreeSet;

use crate::circuit::{Circuit, Generator, Pos, Wire, WireKind};
use crate::simulation::{Simulator, StepResult};

fn make_circuit(cells: &[Pos], wires: Vec<Wire>) -> Circuit {
    Circuit::new(BTreeSet::from_iter(cells.iter().copied()), wires).expect("valid circuit")
}

fn make_circuit_with_generators(
    cells: &[Pos],
    wires: Vec<Wire>,
    generators: Vec<Generator>,
) -> Circuit {
    Circuit::with_generators(
        BTreeSet::from_iter(cells.iter().copied()),
        wires,
        generators,
    )
    .expect("valid circuit")
}

#[test]
fn positive_chain_propagates_within_one_tick() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)],
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
        &[Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)],
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
        &[Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)],
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

#[test]
fn run_with_snapshots_collects_tick_states() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
    );

    let mut sim = Simulator::new(circuit);
    sim.state_mut()
        .set(Pos::new(0, 0), true)
        .expect("state update must succeed");

    let snapshots = sim.run_with_snapshots(2);

    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].tick, 1);
    assert_eq!(snapshots[1].tick, 2);
    assert_eq!(snapshots[0].cells[0], (Pos::new(0, 0), true));
    assert_eq!(snapshots[0].cells[1], (Pos::new(1, 0), true));
}

#[test]
fn generator_non_loop_holds_last_value() {
    let circuit = make_circuit_with_generators(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
        vec![Generator::new(Pos::new(0, 0), vec![true, false], false)],
    );

    let mut sim = Simulator::new(circuit);
    sim.run(3);

    assert_eq!(sim.state().get(Pos::new(1, 0)), Some(false));
}

#[test]
fn generator_loop_repeats_pattern() {
    let circuit = make_circuit_with_generators(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
        vec![Generator::new(Pos::new(0, 0), vec![true, false], true)],
    );

    let mut sim = Simulator::new(circuit);
    sim.run(3);

    assert_eq!(sim.state().get(Pos::new(1, 0)), Some(true));
}

#[test]
fn generator_is_applied_when_stepping_cell_by_cell() {
    let circuit = make_circuit_with_generators(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
        vec![Generator::new(Pos::new(0, 0), vec![true], false)],
    );

    let mut sim = Simulator::new(circuit);
    assert_eq!(sim.step(), StepResult::Continue);
    assert_eq!(sim.step(), StepResult::TickComplete);

    assert_eq!(sim.state().get(Pos::new(1, 0)), Some(true));
}

#[test]
fn circuit_accessor_returns_original_circuit() {
    let cells = [Pos::new(0, 0), Pos::new(1, 0)];
    let wires = vec![Wire::new(
        Pos::new(0, 0),
        Pos::new(1, 0),
        WireKind::Positive,
    )];
    let circuit = make_circuit(&cells, wires);
    let sim = Simulator::new(circuit.clone());

    assert_eq!(sim.circuit().sorted_cells(), circuit.sorted_cells());
    assert_eq!(sim.circuit().wires(), circuit.wires());
}
