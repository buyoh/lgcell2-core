use std::collections::BTreeSet;

use lgcell2_core::circuit::{Circuit, Pos, Wire, WireKind};
use lgcell2_core::simulation::Simulator;

fn run_half_adder(a: bool, b: bool) -> (bool, bool) {
    let mut sim = Simulator::new(build_half_adder());

    // 初期値は全て 0 なので、入力セルの値を手動で設定してから tick する
    sim.state_mut().set(Pos::new(0, 0), a).unwrap();
    sim.state_mut().set(Pos::new(0, 1), b).unwrap();
    sim.tick();

    let sum = sim
        .state()
        .get(Pos::new(3, 0))
        .expect("sum cell must exist");
    let carry = sim
        .state()
        .get(Pos::new(3, 1))
        .expect("carry cell must exist");

    (sum, carry)
}

fn build_half_adder() -> Circuit {
    let cells = BTreeSet::from([
        Pos::new(0, 0), // a
        Pos::new(0, 1), // b
        Pos::new(1, 0), // or1
        Pos::new(1, 1), // nand1
        Pos::new(2, 0), // nand_xor
        Pos::new(3, 0), // sum
        Pos::new(2, 1), // nand_ab
        Pos::new(3, 1), // carry
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
        let (sum, carry) = run_half_adder(a, b);
        assert_eq!(sum, expected_sum);
        assert_eq!(carry, expected_carry);
    }
}

#[test]
fn half_adder_is_stateless_under_alternating_inputs() {
    for _ in 0..32 {
        let (sum_11, carry_11) = run_half_adder(true, true);
        assert_eq!(sum_11, false);
        assert_eq!(carry_11, true);

        let (sum_00, carry_00) = run_half_adder(false, false);
        assert_eq!(sum_00, false);
        assert_eq!(carry_00, false);
    }
}
