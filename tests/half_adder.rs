use std::collections::BTreeMap;

use lgcell2_core::circuit::{Circuit, Pos, Wire, WireKind};
use lgcell2_core::simulation::Simulator;

fn build_half_adder(a: bool, b: bool) -> Circuit {
    let cells = BTreeMap::from([
        (Pos::new(0, 0), a),
        (Pos::new(0, 1), b),
        (Pos::new(1, 0), false), // or1
        (Pos::new(1, 1), false), // nand1
        (Pos::new(2, 0), false), // nand_xor
        (Pos::new(3, 0), false), // sum
        (Pos::new(2, 1), false), // nand_ab
        (Pos::new(3, 1), false), // carry
    ]);

    let wires = vec![
        Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive),
        Wire::new(Pos::new(0, 1), Pos::new(1, 0), WireKind::Positive),
        Wire::new(Pos::new(0, 0), Pos::new(1, 1), WireKind::Negative),
        Wire::new(Pos::new(0, 1), Pos::new(1, 1), WireKind::Negative),
        Wire::new(Pos::new(1, 0), Pos::new(2, 0), WireKind::Negative),
        Wire::new(Pos::new(1, 1), Pos::new(2, 0), WireKind::Negative),
        Wire::new(Pos::new(2, 0), Pos::new(3, 0), WireKind::Negative),
        Wire::new(Pos::new(0, 0), Pos::new(2, 1), WireKind::Negative),
        Wire::new(Pos::new(0, 1), Pos::new(2, 1), WireKind::Negative),
        Wire::new(Pos::new(2, 1), Pos::new(3, 1), WireKind::Negative),
    ];

    Circuit::new(cells, wires).expect("half adder circuit must be valid")
}

#[test]
fn half_adder_truth_table() {
    let cases = [
        (false, false, false, false),
        (false, true, true, false),
        (true, false, true, false),
        (true, true, false, true),
    ];

    for (a, b, expected_sum, expected_carry) in cases {
        let mut sim = Simulator::new(build_half_adder(a, b));
        sim.tick();

        assert_eq!(sim.state().get(Pos::new(3, 0)), Some(expected_sum));
        assert_eq!(sim.state().get(Pos::new(3, 1)), Some(expected_carry));
    }
}
