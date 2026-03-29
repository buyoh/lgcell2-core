use crate::io::json::{
    CircuitJson, output_json_to_string, parse_circuit_json, simulate_to_output_json,
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
    assert_eq!(sim.get_cell(crate::circuit::Pos::new(1, 0)), Some(true));
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
    assert!(
        matches!(err, crate::base::ParseError::Format(crate::base::FormatError::InvalidWireKind(ref kind)) if kind == "unknown")
    );
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
    assert!(matches!(
        err,
        crate::base::ParseError::Circuit(crate::base::CircuitError::SelfLoop {
            src: crate::circuit::Pos { x: 0, y: 0 },
            dst: crate::circuit::Pos { x: 0, y: 0 }
        })
    ));
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
    assert!(matches!(
        err,
        crate::base::ParseError::Circuit(crate::base::CircuitError::DuplicateWire {
            src: crate::circuit::Pos { x: 0, y: 0 },
            dst: crate::circuit::Pos { x: 1, y: 0 }
        })
    ));
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
      "input": [
        { "type": "generator", "target": [0, 0], "pattern": "10" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert_eq!(circuit.inputs().len(), 1);
    match &circuit.inputs()[0] {
        crate::circuit::Input::Generator(generator) => {
            assert_eq!(generator.target(), crate::circuit::Pos::new(0, 0));
            assert_eq!(generator.pattern(), &[true, false]);
            assert!(!generator.is_loop());
        }
    }
}

#[test]
fn parse_rejects_invalid_generator_pattern_char() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
      ],
      "input": [
        { "type": "generator", "target": [0, 0], "pattern": "10x" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject invalid pattern character");
    assert!(matches!(
        err,
        crate::base::ParseError::Format(crate::base::FormatError::InvalidPatternChar('x'))
    ));
}

#[test]
fn circuit_json_deserializes_without_generators() {
    let input = r#"
    {
      "wires": []
    }
    "#;

    let parsed: CircuitJson = serde_json::from_str(input).expect("must deserialize");
    assert!(parsed.input.is_empty());
    assert!(parsed.generators.is_empty());
}

  #[test]
  fn circuit_json_deserializes_legacy_generators_for_compatibility() {
    let input = r#"
    {
      "wires": [],
      "generators": [
      { "target": [0, 0], "pattern": "10" }
      ]
    }
    "#;

    let parsed: CircuitJson = serde_json::from_str(input).expect("must deserialize");
    assert_eq!(parsed.generators.len(), 1);
  }

  #[test]
  fn parse_expected_pattern_returns_invalid_expected_pattern_char_error() {
    use crate::io::json::parse_expected_pattern;

    let err = parse_expected_pattern("10a").expect_err("must reject invalid expected pattern");
    assert!(matches!(
      err,
      crate::base::FormatError::InvalidExpectedPatternChar('a')
    ));
  }

#[test]
fn parse_wire_kind_returns_format_error() {
    use crate::io::json::parse_wire_kind;
    let err = parse_wire_kind("bad").expect_err("must reject unknown kind");
    assert!(matches!(err, crate::base::FormatError::InvalidWireKind(ref s) if s == "bad"));
}

#[test]
fn parse_pattern_returns_invalid_pattern_char_error() {
    use crate::io::json::parse_pattern;
    let err = parse_pattern("01a").expect_err("must reject invalid pattern char");
    assert!(matches!(
        err,
        crate::base::FormatError::InvalidPatternChar('a')
    ));
}

#[test]
fn parse_pattern_returns_error_for_first_invalid_char() {
    use crate::io::json::parse_pattern;
    let err = parse_pattern("z10").expect_err("must reject invalid pattern char");
    assert!(matches!(
        err,
        crate::base::FormatError::InvalidPatternChar('z')
    ));
}
