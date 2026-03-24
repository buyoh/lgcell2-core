use crate::io::json::{
    output_json_to_string, parse_circuit_json, simulate_to_output_json, CircuitJson,
};
use crate::simulation::Simulator;

#[test]
fn parse_valid_json_to_circuit() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    let mut sim = Simulator::new(circuit);
    sim.tick();

    // src=0,0 は初期値 false → Negative で反転 → true
    assert_eq!(sim.state().get(crate::circuit::Pos::new(1, 0)), Some(true));
}

#[test]
fn parse_rejects_invalid_kind() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "unknown" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject unknown kind");
    assert!(err.contains("wire kind must be positive or negative"));
}

#[test]
fn parse_rejects_self_loop() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [0, 0], "kind": "positive" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject self-loop");
    assert!(err.contains("self-loop wire is not allowed"));
}

#[test]
fn parse_rejects_duplicate_wires() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
        { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject duplicate wires");
    assert!(err.contains("duplicate wire is not allowed"));
}

#[test]
fn output_json_has_expected_shape() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    let output = simulate_to_output_json(circuit, 2);
    let text = output_json_to_string(&output).expect("serialization must succeed");

    assert!(text.contains("\"ticks\""));
    assert!(text.contains("\"tick\": 1"));
    assert!(text.contains("\"0,0\": 0"));
    assert!(text.contains("\"1,0\": 1"));
}

#[test]
fn circuit_json_deserializes() {
    let input = r#"
    {
      "wires": []
    }
    "#;

    let parsed: CircuitJson = serde_json::from_str(input).expect("must deserialize");
    assert!(parsed.wires.is_empty());
}

#[test]
fn cells_are_inferred_from_wire_endpoints() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
        { "src": [1, 0], "dst": [2, 0], "kind": "negative" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert_eq!(circuit.cells().len(), 3);
    assert!(circuit.cells().contains(&crate::circuit::Pos::new(0, 0)));
    assert!(circuit.cells().contains(&crate::circuit::Pos::new(1, 0)));
    assert!(circuit.cells().contains(&crate::circuit::Pos::new(2, 0)));
}

#[test]
fn parse_generators_and_apply_default_loop_false() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
      ],
      "generators": [
        { "target": [0, 0], "pattern": "10" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert_eq!(circuit.generators().len(), 1);
    assert_eq!(circuit.generators()[0].target(), crate::circuit::Pos::new(0, 0));
    assert_eq!(circuit.generators()[0].pattern(), &[true, false]);
    assert!(!circuit.generators()[0].is_loop());
}

#[test]
fn parse_rejects_invalid_generator_pattern_char() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
      ],
      "generators": [
        { "target": [0, 0], "pattern": "10x" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject invalid pattern character");
    assert!(err.contains("invalid pattern character"));
}

#[test]
fn circuit_json_deserializes_without_generators() {
    let input = r#"
    {
      "wires": []
    }
    "#;

    let parsed: CircuitJson = serde_json::from_str(input).expect("must deserialize");
    assert!(parsed.generators.is_empty());
}
