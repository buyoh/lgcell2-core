use std::collections::BTreeSet;

use crate::circuit::{Circuit, Pos, ResolvedModule, Wire, WireKind};

#[test]
fn resolved_module_accessors() {
    let sub_cells = BTreeSet::from([Pos::new(0, 0), Pos::new(1, 0)]);
    let sub_wires = vec![Wire::new(Pos::new(0, 0), Pos::new(1, 0), WireKind::Negative)];
    let sub_circuit = Circuit::new(sub_cells, sub_wires).expect("sub-circuit must be valid");

    let module = ResolvedModule::new(
        sub_circuit.clone(),
        vec![Pos::new(1, 0)],
        vec![Pos::new(2, 0)],
        vec![Pos::new(0, 0)],
        vec![Pos::new(1, 0)],
    );

    assert_eq!(module.input(), &[Pos::new(1, 0)]);
    assert_eq!(module.output(), &[Pos::new(2, 0)]);
    assert_eq!(module.sub_input(), &[Pos::new(0, 0)]);
    assert_eq!(module.sub_output(), &[Pos::new(1, 0)]);
    assert_eq!(module.circuit().wires().len(), 1);
}
