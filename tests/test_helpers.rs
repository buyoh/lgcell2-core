use std::collections::{BTreeMap, BTreeSet};

use lgcell2_core::base::ParseError;
use lgcell2_core::circuit::{Circuit, Generator, Pos, Wire, WireKind};
use lgcell2_core::io::json::CircuitJson;
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
        .unwrap_or_else(|| panic!("Test case '{}' not found in {}/check.json", case_name, test_dir));

    // 3. circuit.json の generator と case.generator を target 単位でマージ
    let circuit = build_circuit_with_case_generators(&circuit_json, &test_case.generators)
        .unwrap_or_else(|e| panic!("Failed to build circuit for test case {}: {}", case_name, e));

    // 4. Simulator を作成して初期値を設定
    let mut sim = Simulator::new(circuit);

    for (pos_str, value) in &test_case.initial {
        let pos = parse_pos(pos_str);

        sim.state_mut()
            .set(pos, *value)
            .unwrap_or_else(|e| panic!("Failed to set value at {}: {}", pos_str, e));
    }

    // 5. ticks 回実行（case に指定があれば優先）
    let ticks = test_case.ticks.unwrap_or(check.ticks) as u64;
    sim.run(ticks);

    // 6. expected の各値を検証
    for (pos_str, expected_value) in &test_case.expected {
        let pos = parse_pos(pos_str);

        let actual_value = sim
            .state()
            .get(pos)
            .expect(&format!("Failed to get value at {}", pos_str));
        assert_eq!(
            actual_value, *expected_value,
            "Mismatch at {} in test case {}: expected {}, got {}",
            pos_str, case_name, expected_value, actual_value
        );
    }
}

fn build_circuit_with_case_generators(
    circuit_json: &CircuitJson,
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

    let mut generators_by_target: BTreeMap<Pos, Generator> = BTreeMap::new();

    for generator in &circuit_json.generators {
        let target = Pos::new(generator.target[0], generator.target[1]);
        let pattern = parse_pattern(&generator.pattern)?;
        generators_by_target.insert(target, Generator::new(target, pattern, generator.is_loop));
    }

    for generator in case_generators {
        let target = Pos::new(generator.target[0], generator.target[1]);
        let pattern = parse_pattern(&generator.pattern)?;
        generators_by_target.insert(target, Generator::new(target, pattern, generator.is_loop));
    }

    let generators = generators_by_target.into_values().collect::<Vec<_>>();
    Circuit::with_generators(cells, wires, generators).map_err(ParseError::from)
}

fn parse_wire_kind(kind: &str) -> Result<WireKind, ParseError> {
    match kind {
        "positive" => Ok(WireKind::Positive),
        "negative" => Ok(WireKind::Negative),
        _ => Err(ParseError::InvalidWireKind(kind.to_string())),
    }
}

fn parse_pattern(pattern: &str) -> Result<Vec<bool>, ParseError> {
    pattern
        .chars()
        .map(|c| match c {
            '1' => Ok(true),
            '0' => Ok(false),
            _ => Err(ParseError::InvalidWireKind(format!(
                "invalid pattern character: '{}' (expected '0' or '1')",
                c
            ))),
        })
        .collect()
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
