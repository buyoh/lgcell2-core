use lgcell2_core::circuit::Pos;
use lgcell2_core::simulation::Simulator;

/// シミュレーション型テストケースを実行する
pub fn test_simulation_case(test_dir: &str, case_name: &str) {
    // 1. circuit.json を読み込みパース
    let circuit_path = format!("resources/tests/{}/circuit.json", test_dir);
    let circuit_content = std::fs::read_to_string(&circuit_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", circuit_path));

    let circuit = lgcell2_core::io::json::parse_circuit_json(&circuit_content)
        .unwrap_or_else(|e| panic!("Failed to parse circuit from {}/circuit.json: {}", test_dir, e));

    // 2. check.json を読み込みテストケースを取得
    let check_path = format!("resources/tests/{}/check.json", test_dir);
    let check_content = std::fs::read_to_string(&check_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", check_path));

    #[derive(serde::Deserialize)]
    struct CheckFile {
        ticks: usize,
        cases: Vec<TestCase>,
    }

    #[derive(serde::Deserialize)]
    struct TestCase {
        name: String,
        #[serde(default)]
        initial: std::collections::BTreeMap<String, bool>,
        #[serde(default)]
        expected: std::collections::BTreeMap<String, bool>,
    }

    let check: CheckFile = serde_json::from_str(&check_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}/check.json", test_dir));

    // ケース名に一致するテストケースを取得
    let test_case = check
        .cases
        .iter()
        .find(|c| c.name == case_name)
        .unwrap_or_else(|| panic!("Test case '{}' not found in {}/check.json", case_name, test_dir));

    // 3. Simulator を作成して初期値を設定
    let mut sim = Simulator::new(circuit);

    for (pos_str, value) in &test_case.initial {
        let parts: Vec<&str> = pos_str.split(',').collect();
        assert!(parts.len() == 2, "Invalid position format: {}", pos_str);
        let x: i32 = parts[0].trim().parse().expect("Invalid x coordinate");
        let y: i32 = parts[1].trim().parse().expect("Invalid y coordinate");

        sim.state_mut()
            .set(Pos::new(x, y), *value)
            .unwrap_or_else(|e| panic!("Failed to set value at {}: {}", pos_str, e));
    }

    // 4. ticks 回実行
    for _ in 0..check.ticks {
        sim.tick();
    }

    // 5. expected の各値を検証
    for (pos_str, expected_value) in &test_case.expected {
        let parts: Vec<&str> = pos_str.split(',').collect();
        assert!(parts.len() == 2, "Invalid position format: {}", pos_str);
        let x: i32 = parts[0].trim().parse().expect("Invalid x coordinate");
        let y: i32 = parts[1].trim().parse().expect("Invalid y coordinate");


        let actual_value = sim
            .state()
            .get(Pos::new(x, y))
            .expect(&format!("Failed to get value at {}", pos_str));
        assert_eq!(
            actual_value, *expected_value,
            "Mismatch at {} in test case {}: expected {}, got {}",
            pos_str, case_name, expected_value, actual_value
        );
    }
}
