use std::collections::{BTreeMap, BTreeSet};

use lgcell2_core::base::ParseError;
use lgcell2_core::circuit::{Circuit, Generator, Input, Output, Pos, Tester, Wire};
use lgcell2_core::io::json::{
    CircuitJson, InputJson, OutputJson, parse_expected_pattern, parse_pattern, parse_wire_kind,
};
use lgcell2_core::simulation::Simulator;

#[derive(serde::Deserialize)]
struct CheckFile {
    ticks: usize,
    cases: Vec<TestCase>,
}

#[derive(serde::Deserialize)]
struct TestCase {
    name: String,
    #[serde(default)]
    ticks: Option<usize>,
    #[serde(default)]
    initial: BTreeMap<String, bool>,
    #[serde(default)]
    input: Vec<InputCaseJson>,
    #[serde(default)]
    output: Vec<OutputCaseJson>,
    #[serde(default)]
    generators: Vec<GeneratorJson>,
    #[serde(default)]
    expected: BTreeMap<String, bool>,
}

#[derive(serde::Deserialize)]
struct GeneratorJson {
    target: [i32; 2],
    pattern: String,
    #[serde(default, rename = "loop")]
    is_loop: bool,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InputCaseJson {
    Generator {
        target: [i32; 2],
        pattern: String,
        #[serde(default, rename = "loop")]
        is_loop: bool,
    },
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OutputCaseJson {
    Tester {
        target: [i32; 2],
        expected: String,
        #[serde(default, rename = "loop")]
        is_loop: bool,
    },
}

/// シミュレーション型テストケースを実行する
pub fn test_simulation_case(test_dir: &str, case_name: &str) {
    // 1. circuit.json を読み込み JSON モデルとしてパース
    let circuit_path = format!("resources/tests/{}/circuit.json", test_dir);
    let circuit_content = std::fs::read_to_string(&circuit_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", circuit_path));

    let circuit_json: CircuitJson = serde_json::from_str(&circuit_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}", circuit_path));

    // 2. check.json を読み込みテストケースを取得
    let check_path = format!("resources/tests/{}/check.json", test_dir);
    let check_content = std::fs::read_to_string(&check_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", check_path));

    let check: CheckFile = serde_json::from_str(&check_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}/check.json", test_dir));

    // ケース名に一致するテストケースを取得
    let test_case = check
        .cases
        .iter()
        .find(|c| c.name == case_name)
        .unwrap_or_else(|| {
            panic!(
                "Test case '{}' not found in {}/check.json",
                case_name, test_dir
            )
        });

    // 3. circuit.json の input と case.input を target 単位でマージ
    let circuit = build_circuit_with_case_inputs(
        &circuit_json,
        &test_case.input,
        &test_case.output,
        &test_case.generators,
    )
        .unwrap_or_else(|e| panic!("Failed to build circuit for test case {}: {}", case_name, e));

    // 4. Simulator を作成して初期値を設定
    let mut sim = Simulator::new(circuit);

    for (pos_str, value) in &test_case.initial {
        let pos = parse_pos(pos_str);

        sim.set_cell(pos, *value)
            .unwrap_or_else(|e| panic!("Failed to set value at {}: {}", pos_str, e));
    }

    // 5. ticks 回実行（case に指定があれば優先）
    let ticks = test_case.ticks.unwrap_or(check.ticks) as u64;
    let mismatches = sim.run_with_verification(ticks);
    assert!(
        mismatches.is_empty(),
        "Tester mismatches found in test case {}: {:?}",
        case_name,
        mismatches
    );

    // 6. expected の各値を検証
    for (pos_str, expected_value) in &test_case.expected {
        let pos = parse_pos(pos_str);

        let actual_value = sim
            .get_cell(pos)
            .unwrap_or_else(|| panic!("Failed to get value at {}", pos_str));
        assert_eq!(
            actual_value, *expected_value,
            "Mismatch at {} in test case {}: expected {}, got {}",
            pos_str, case_name, expected_value, actual_value
        );
    }
}

fn build_circuit_with_case_inputs(
    circuit_json: &CircuitJson,
    case_inputs: &[InputCaseJson],
    case_outputs: &[OutputCaseJson],
    case_generators: &[GeneratorJson],
) -> Result<Circuit, ParseError> {
    let mut cells = BTreeSet::new();
    let mut wires = Vec::with_capacity(circuit_json.wires.len());

    for wire in &circuit_json.wires {
        let src = Pos::new(wire.src[0], wire.src[1]);
        let dst = Pos::new(wire.dst[0], wire.dst[1]);
        let kind = parse_wire_kind(&wire.kind)?;

        cells.insert(src);
        cells.insert(dst);
        wires.push(Wire::new(src, dst, kind));
    }

    let mut inputs_by_target: BTreeMap<Pos, Input> = BTreeMap::new();

    for input in &circuit_json.input {
        match input {
            InputJson::Generator {
                target,
                pattern,
                is_loop,
            } => {
                let target = Pos::new(target[0], target[1]);
                let pattern = parse_pattern(pattern)?;
                inputs_by_target.insert(
                    target,
                    Input::Generator(Generator::new(target, pattern, *is_loop)),
                );
            }
        }
    }

    for generator in &circuit_json.generators {
        let target = Pos::new(generator.target[0], generator.target[1]);
        let pattern = parse_pattern(&generator.pattern)?;
        inputs_by_target.insert(
            target,
            Input::Generator(Generator::new(target, pattern, generator.is_loop)),
        );
    }

    for input in case_inputs {
        match input {
            InputCaseJson::Generator {
                target,
                pattern,
                is_loop,
            } => {
                let target = Pos::new(target[0], target[1]);
                let pattern = parse_pattern(pattern)?;
                inputs_by_target.insert(
                    target,
                    Input::Generator(Generator::new(target, pattern, *is_loop)),
                );
            }
        }
    }

    for generator in case_generators {
        let target = Pos::new(generator.target[0], generator.target[1]);
        let pattern = parse_pattern(&generator.pattern)?;
        inputs_by_target.insert(
            target,
            Input::Generator(Generator::new(target, pattern, generator.is_loop)),
        );
    }

    let mut outputs_by_target: BTreeMap<Pos, Output> = BTreeMap::new();
    for output in &circuit_json.output {
        match output {
            OutputJson::Tester {
                target,
                expected,
                is_loop,
            } => {
                let target = Pos::new(target[0], target[1]);
                let expected = parse_expected_pattern(expected)?;
                outputs_by_target.insert(
                    target,
                    Output::Tester(Tester::new(target, expected, *is_loop)),
                );
            }
        }
    }

    for tester in &circuit_json.testers {
        let target = Pos::new(tester.target[0], tester.target[1]);
        let expected = parse_expected_pattern(&tester.expected)?;
        outputs_by_target.insert(
            target,
            Output::Tester(Tester::new(target, expected, tester.is_loop)),
        );
    }

    for output in case_outputs {
        match output {
            OutputCaseJson::Tester {
                target,
                expected,
                is_loop,
            } => {
                let target = Pos::new(target[0], target[1]);
                let expected = parse_expected_pattern(expected)?;
                outputs_by_target.insert(
                    target,
                    Output::Tester(Tester::new(target, expected, *is_loop)),
                );
            }
        }
    }

    let inputs = inputs_by_target.into_values().collect::<Vec<_>>();
    let outputs = outputs_by_target.into_values().collect::<Vec<_>>();
    Circuit::with_components(cells, wires, inputs, outputs).map_err(ParseError::from)
}

fn parse_pos(pos_str: &str) -> Pos {
    let parts: Vec<&str> = pos_str.split(',').collect();
    assert!(parts.len() == 2, "Invalid position format: {}", pos_str);

    let x: i32 = parts[0].trim().parse().expect("Invalid x coordinate");
    let y: i32 = parts[1].trim().parse().expect("Invalid y coordinate");
    Pos::new(x, y)
}

/// validation 型テストケースを実行する。
pub fn test_validation_case(test_dir: &str) {
    let circuit_path = format!("resources/tests/{}/circuit.json", test_dir);
    let circuit_content = std::fs::read_to_string(&circuit_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", circuit_path));

    let expected_path = format!("resources/tests/{}/expected.json", test_dir);
    let expected_content = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", expected_path));

    #[derive(serde::Deserialize)]
    struct ExpectedError {
        error_contains: String,
    }

    let expected: ExpectedError = serde_json::from_str(&expected_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}", expected_path));

    let result = lgcell2_core::io::json::parse_circuit_json(&circuit_content);
    let err = result.expect_err(&format!(
        "Circuit in {} should be rejected, but was accepted",
        test_dir
    ));
    let err_msg = err.to_string();
    assert!(
        err_msg.contains(&expected.error_contains),
        "Error message '{}' does not contain expected substring '{}'",
        err_msg,
        expected.error_contains
    );
}
