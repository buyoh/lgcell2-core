use crate::parser::json::{
    CircuitJson, output_json_to_string, parse_circuit_json, simulate_to_output_json,
};
use crate::simulation::{Simulator, SimulatorSimple};

fn output_cell(sim: &SimulatorSimple, pos: crate::circuit::Pos) -> Option<bool> {
    sim.last_output().cells.get(&pos).copied()
}

fn sim_cell_at(sim: &mut SimulatorSimple, pos: crate::circuit::Pos, ticks: u64) -> bool {
    sim.run(ticks);
    output_cell(sim, pos).unwrap_or(false)
}

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
    let mut sim = SimulatorSimple::new(circuit);
    sim.tick();

    // src=0,0 は初期値 false → Negative で反転 → true
    assert_eq!(
        output_cell(&sim, crate::circuit::Pos::new(1, 0)),
        Some(true)
    );
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
    assert!(text.contains("\"tick\": 0"));
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
fn circuit_json_deserializes_without_input() {
    let input = r#"
    {
      "wires": []
    }
    "#;

    let parsed: CircuitJson = serde_json::from_str(input).expect("must deserialize");
    assert!(parsed.input.is_empty());
}

#[test]
fn legacy_generators_field_is_silently_ignored() {
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
    assert!(circuit.inputs().is_empty(), "legacy generators must be ignored");
}

#[test]
fn legacy_testers_field_is_silently_ignored() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
      ],
      "testers": [
        { "target": [1, 0], "expected": "0" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert!(circuit.outputs().is_empty(), "legacy testers must be ignored");
}

#[test]
fn parse_expected_pattern_returns_invalid_expected_pattern_char_error() {
    use crate::parser::json::parse_expected_pattern;

    let err = parse_expected_pattern("10a").expect_err("must reject invalid expected pattern");
    assert!(matches!(
        err,
        crate::base::FormatError::InvalidExpectedPatternChar('a')
    ));
}

#[test]
fn parse_wire_kind_returns_format_error() {
    use crate::parser::json::parse_wire_kind;
    let err = parse_wire_kind("bad").expect_err("must reject unknown kind");
    assert!(matches!(err, crate::base::FormatError::InvalidWireKind(ref s) if s == "bad"));
}

#[test]
fn parse_pattern_returns_invalid_pattern_char_error() {
    use crate::parser::json::parse_pattern;
    let err = parse_pattern("01a").expect_err("must reject invalid pattern char");
    assert!(matches!(
        err,
        crate::base::FormatError::InvalidPatternChar('a')
    ));
}

#[test]
fn parse_pattern_returns_error_for_first_invalid_char() {
    use crate::parser::json::parse_pattern;
    let err = parse_pattern("z10").expect_err("must reject invalid pattern char");
    assert!(matches!(
        err,
        crate::base::FormatError::InvalidPatternChar('z')
    ));
}

// --- Sub-circuit JSON parsing tests ---

#[test]
fn parse_sub_circuit_single_module() {
    // NOT ゲートをサブ回路として定義し、モジュールとして使用
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "inverter",
          "input": [ [0, 0] ],
          "output": [ [1, 0] ]
        }
      ],
      "subs": {
        "inverter": {
          "wires": [
            { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
          ],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ]
        }
      }
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert_eq!(circuit.modules().len(), 1);
}

#[test]
fn parse_sub_circuit_simulation_inverter() {
    // 入力 false → NOT → true
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "inverter",
          "input": [ [0, 0] ],
          "output": [ [1, 0] ]
        }
      ],
      "subs": {
        "inverter": {
          "wires": [
            { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
          ],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ]
        }
      }
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    let mut sim = SimulatorSimple::new(circuit);
    sim.tick();

    // 入力 (0,0) = false → NOT → 出力 (1,0) = true
    assert_eq!(output_cell(&sim, crate::circuit::Pos::new(1, 0)), Some(true));
}

#[test]
fn parse_sub_circuit_half_adder() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "negative" },
        { "src": [0, 0], "dst": [1, 1], "kind": "negative" }
      ],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "half_adder",
          "input": [ [1, 0], [1, 1] ],
          "output": [ [2, 0], [2, 1] ]
        }
      ],
      "subs": {
        "half_adder": {
          "wires": [
            { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
            { "src": [0, 1], "dst": [1, 0], "kind": "positive" },
            { "src": [0, 0], "dst": [1, 1], "kind": "negative" },
            { "src": [0, 1], "dst": [1, 1], "kind": "negative" },
            { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
            { "src": [1, 1], "dst": [2, 0], "kind": "negative" },
            { "src": [2, 0], "dst": [3, 0], "kind": "negative" },
            { "src": [0, 0], "dst": [2, 1], "kind": "negative" },
            { "src": [0, 1], "dst": [2, 1], "kind": "negative" },
            { "src": [2, 1], "dst": [3, 1], "kind": "negative" }
          ],
          "sub_input": [ [0, 0], [0, 1] ],
          "sub_output": [ [3, 0], [3, 1] ]
        }
      }
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert_eq!(circuit.modules().len(), 1);
}

#[test]
fn parse_sub_circuit_nested_modules() {
    // full_adder uses half_adder
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "full_adder",
          "input": [ [0, 0], [0, 1], [0, 2] ],
          "output": [ [5, 0], [5, 1] ]
        }
      ],
      "subs": {
        "half_adder": {
          "wires": [
            { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
            { "src": [0, 1], "dst": [1, 0], "kind": "positive" },
            { "src": [0, 0], "dst": [1, 1], "kind": "negative" },
            { "src": [0, 1], "dst": [1, 1], "kind": "negative" },
            { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
            { "src": [1, 1], "dst": [2, 0], "kind": "negative" },
            { "src": [2, 0], "dst": [3, 0], "kind": "negative" },
            { "src": [0, 0], "dst": [2, 1], "kind": "negative" },
            { "src": [0, 1], "dst": [2, 1], "kind": "negative" },
            { "src": [2, 1], "dst": [3, 1], "kind": "negative" }
          ],
          "sub_input": [ [0, 0], [0, 1] ],
          "sub_output": [ [3, 0], [3, 1] ]
        },
        "full_adder": {
          "wires": [
            { "src": [3, 0], "dst": [4, 0], "kind": "positive" },
            { "src": [0, 2], "dst": [4, 1], "kind": "positive" },
            { "src": [3, 1], "dst": [8, 1], "kind": "positive" },
            { "src": [7, 1], "dst": [8, 1], "kind": "positive" }
          ],
          "sub_input": [ [0, 0], [0, 1], [0, 2] ],
          "sub_output": [ [8, 0], [8, 1] ],
          "modules": [
            {
              "type": "sub",
              "sub_circuit": "half_adder",
              "input": [ [0, 0], [0, 1] ],
              "output": [ [3, 0], [3, 1] ]
            },
            {
              "type": "sub",
              "sub_circuit": "half_adder",
              "input": [ [4, 0], [4, 1] ],
              "output": [ [7, 0], [7, 1] ]
            }
          ]
        }
      }
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert_eq!(circuit.modules().len(), 1);
    // The single module (full_adder) should itself contain nested modules
    assert_eq!(circuit.modules()[0].circuit().modules().len(), 2);
}

#[test]
fn parse_sub_circuit_rejects_missing_sub() {
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "nonexistent",
          "input": [ [0, 0] ],
          "output": [ [1, 0] ]
        }
      ],
      "subs": {}
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject missing sub-circuit");
    assert!(matches!(err, crate::base::ParseError::SubCircuitNotFound(ref name) if name == "nonexistent"));
}

#[test]
fn parse_sub_circuit_rejects_circular_dependency() {
    let input = r#"
    {
      "wires": [],
      "modules": [],
      "subs": {
        "a": {
          "wires": [],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ],
          "modules": [
            { "type": "sub", "sub_circuit": "b", "input": [ [0, 0] ], "output": [ [1, 0] ] }
          ]
        },
        "b": {
          "wires": [],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ],
          "modules": [
            { "type": "sub", "sub_circuit": "a", "input": [ [0, 0] ], "output": [ [1, 0] ] }
          ]
        }
      }
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject circular dependency");
    assert!(matches!(err, crate::base::ParseError::CircularDependency(_)));
}

#[test]
fn parse_sub_circuit_rejects_input_count_mismatch() {
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "inv",
          "input": [ [0, 0], [0, 1] ],
          "output": [ [1, 0] ]
        }
      ],
      "subs": {
        "inv": {
          "wires": [
            { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
          ],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ]
        }
      }
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject input count mismatch");
    assert!(matches!(
        err,
        crate::base::ParseError::Circuit(crate::base::CircuitError::SubInputCountMismatch {
            expected: 1,
            actual: 2
        })
    ));
}

#[test]
fn parse_sub_circuit_rejects_output_count_mismatch() {
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "inv",
          "input": [ [0, 0] ],
          "output": [ [1, 0], [1, 1] ]
        }
      ],
      "subs": {
        "inv": {
          "wires": [
            { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
          ],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ]
        }
      }
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject output count mismatch");
    assert!(matches!(
        err,
        crate::base::ParseError::Circuit(crate::base::CircuitError::SubOutputCountMismatch {
            expected: 1,
            actual: 2
        })
    ));
}

#[test]
fn parse_sub_circuit_rejects_sub_input_with_incoming_wire() {
    let input = r#"
    {
      "wires": [],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "bad",
          "input": [ [0, 0] ],
          "output": [ [1, 0] ]
        }
      ],
      "subs": {
        "bad": {
          "wires": [
            { "src": [1, 0], "dst": [0, 0], "kind": "positive" }
          ],
          "sub_input": [ [0, 0] ],
          "sub_output": [ [1, 0] ]
        }
      }
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject sub_input with incoming wire");
    assert!(matches!(
        err,
        crate::base::ParseError::Circuit(crate::base::CircuitError::SubInputHasIncomingWires(_))
    ));
}

#[test]
fn parse_sub_circuit_backward_compatible_without_subs() {
    let input = r#"
    {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    assert!(circuit.modules().is_empty());
    let mut sim = SimulatorSimple::new(circuit);
    sim.tick();
    assert_eq!(output_cell(&sim, crate::circuit::Pos::new(1, 0)), Some(true));
}
