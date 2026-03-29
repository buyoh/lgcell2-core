use std::collections::BTreeSet;

use crate::circuit::{Circuit, Generator, Input, Output, Pos, Tester, Wire, WireKind};
use crate::simulation::{OutputFormat, Rect, Simulator, StepResult};

fn output_cell(sim: &Simulator, pos: Pos) -> Option<bool> {
    sim.last_output().cells.get(&pos).copied()
}

fn make_circuit(cells: &[Pos], wires: Vec<Wire>) -> Circuit {
    Circuit::new(BTreeSet::from_iter(cells.iter().copied()), wires).expect("valid circuit")
}

fn make_circuit_with_generators(
    cells: &[Pos],
    wires: Vec<Wire>,
    generators: Vec<Generator>,
) -> Circuit {
    let inputs = generators
        .into_iter()
        .map(Input::Generator)
        .collect::<Vec<_>>();
    Circuit::with_components(
        BTreeSet::from_iter(cells.iter().copied()),
        wires,
        inputs,
        Vec::new(),
    )
    .expect("valid circuit")
}

fn make_circuit_with_components(
    cells: &[Pos],
    wires: Vec<Wire>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
) -> Circuit {
    Circuit::with_components(
        BTreeSet::from_iter(cells.iter().copied()),
        wires,
        inputs,
        outputs,
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

    assert_eq!(output_cell(&sim, Pos::new(1, 0)), Some(false));
    assert_eq!(output_cell(&sim, Pos::new(2, 0)), Some(false));
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
    assert_eq!(output_cell(&sim, Pos::new(0, 0)), Some(false));
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
    assert_eq!(output_cell(&sim, Pos::new(2, 0)), Some(true));
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

    assert_eq!(by_tick.last_output(), by_step.last_output());
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
    sim.set_cell(Pos::new(0, 0), true)
        .expect("state update must succeed");

    let snapshots = sim.run_with_snapshots(2);

    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].tick, 0);
    assert_eq!(snapshots[1].tick, 1);
    assert_eq!(snapshots[0].cells.get(&Pos::new(0, 0)), Some(&true));
    assert_eq!(snapshots[0].cells.get(&Pos::new(1, 0)), Some(&true));
}

#[test]
fn viewport_snapshot_filters_cells() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
    );

    let mut sim = Simulator::with_output_format(
        circuit,
        OutputFormat::ViewPort(vec![Rect::new(Pos::new(1, 0), Pos::new(1, 0))]),
    );
    sim.tick();

    let snapshots = sim.run_with_snapshots(1);
    assert_eq!(snapshots[0].cells.len(), 1);
    assert!(snapshots[0].cells.contains_key(&Pos::new(1, 0)));
}

#[test]
fn set_cell_updates_last_output_immediately() {
    let circuit = make_circuit(&[Pos::new(0, 0)], vec![]);

    let mut sim = Simulator::new(circuit);
    sim.set_cell(Pos::new(0, 0), true)
        .expect("state update must succeed");

    assert_eq!(output_cell(&sim, Pos::new(0, 0)), Some(true));
}

#[test]
fn replay_tick_rebuilds_output_after_output_format_change() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
    );

    let mut sim = Simulator::new(circuit);
    sim.set_cell(Pos::new(0, 0), true)
        .expect("state update must succeed");
    sim.set_output_format(OutputFormat::ViewPort(vec![Rect::new(
        Pos::new(1, 0),
        Pos::new(1, 0),
    )]));

    assert_eq!(sim.last_output().cells.len(), 2);

    sim.replay_tick();

    assert_eq!(sim.last_output().cells.len(), 1);
    assert_eq!(output_cell(&sim, Pos::new(1, 0)), Some(false));
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

    assert_eq!(output_cell(&sim, Pos::new(1, 0)), Some(false));
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

    assert_eq!(output_cell(&sim, Pos::new(1, 0)), Some(true));
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

    assert_eq!(output_cell(&sim, Pos::new(1, 0)), Some(true));
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

#[test]
fn verify_testers_detects_mismatch_after_tick() {
    let circuit = make_circuit_with_components(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
        vec![Input::Generator(Generator::new(
            Pos::new(0, 0),
            vec![false],
            false,
        ))],
        vec![Output::Tester(Tester::new(
            Pos::new(1, 0),
            vec![Some(true)],
            false,
        ))],
    );

    let mut sim = Simulator::new(circuit);
    sim.tick();

    let mismatches = sim.verify_testers();
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].tick, 0);
    assert_eq!(mismatches[0].expected, true);
    assert_eq!(mismatches[0].actual, false);
}

#[test]
fn run_with_verification_collects_all_tick_mismatches() {
    let circuit = make_circuit_with_components(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(
            Pos::new(0, 0),
            Pos::new(1, 0),
            WireKind::Positive,
        )],
        vec![Input::Generator(Generator::new(
            Pos::new(0, 0),
            vec![true, false],
            true,
        ))],
        vec![Output::Tester(Tester::new(
            Pos::new(1, 0),
            vec![Some(false), Some(false)],
            true,
        ))],
    );

    let mut sim = Simulator::new(circuit);
    let mismatches = sim.run_with_verification(2);

    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].tick, 0);
}

#[test]
fn is_updating_false_after_construction() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive)],
    );
    let sim = Simulator::new(circuit);
    assert!(!sim.is_updating());
}

#[test]
fn is_updating_true_during_tick() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive)],
    );
    let mut sim = Simulator::new(circuit);
    let result = sim.step();
    assert_eq!(result, StepResult::Continue);
    assert!(sim.is_updating());
}

#[test]
fn is_updating_false_after_tick_complete() {
    let circuit = make_circuit(
        &[Pos::new(0, 0), Pos::new(1, 0)],
        vec![Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive)],
    );
    let mut sim = Simulator::new(circuit);
    sim.tick();
    assert!(!sim.is_updating());
}
