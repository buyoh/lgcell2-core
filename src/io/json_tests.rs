use crate::io::json::{
    output_json_to_string, parse_circuit_json, simulate_to_output_json, CircuitJson,
};
use crate::simulation::Simulator;

#[test]
fn parse_valid_json_to_circuit() {
    let input = r#"
    {
      "cells": [
        { "x": 0, "y": 0, "initial": 1 },
        { "x": 1, "y": 0, "initial": 0 }
      ],
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    let mut sim = Simulator::new(circuit);
    sim.tick();

    assert_eq!(sim.state().get(crate::circuit::Pos::new(1, 0)), Some(true));
}

#[test]
fn parse_rejects_unknown_wire_reference() {
    let input = r#"
    {
      "cells": [{ "x": 0, "y": 0, "initial": 1 }],
      "wires": [
        { "src": [0, 0], "dst": [2, 0], "kind": "positive" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject unknown destination");
    assert!(err.contains("wire dst does not exist"));
}

#[test]
fn parse_rejects_invalid_kind() {
    let input = r#"
    {
      "cells": [
        { "x": 0, "y": 0, "initial": 1 },
        { "x": 1, "y": 0, "initial": 0 }
      ],
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
      "cells": [{ "x": 0, "y": 0, "initial": 1 }],
      "wires": [
        { "src": [0, 0], "dst": [0, 0], "kind": "positive" }
      ]
    }
    "#;

    let err = parse_circuit_json(input).expect_err("must reject self-loop");
    assert!(err.contains("self-loop wire is not allowed"));
}

#[test]
fn output_json_has_expected_shape() {
    let input = r#"
    {
      "cells": [
        { "x": 0, "y": 0, "initial": 1 },
        { "x": 1, "y": 0, "initial": 0 }
      ],
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
      ]
    }
    "#;

    let circuit = parse_circuit_json(input).expect("json must parse");
    let output = simulate_to_output_json(circuit, 2);
    let text = output_json_to_string(&output).expect("serialization must succeed");

    assert!(text.contains("\"ticks\""));
    assert!(text.contains("\"tick\": 1"));
    assert!(text.contains("\"0,0\": 1"));
    assert!(text.contains("\"1,0\": 1"));
}

#[test]
fn circuit_json_deserializes() {
    let input = r#"
    {
      "cells": [{ "x": 0, "y": 0, "initial": 0 }],
      "wires": []
    }
    "#;

    let parsed: CircuitJson = serde_json::from_str(input).expect("must deserialize");
    assert_eq!(parsed.cells.len(), 1);
    assert!(parsed.wires.is_empty());
}
