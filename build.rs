use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TestManifest {
    tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    #[serde(rename = "type")]
    test_type: String,
    path: String,
    #[serde(default)]
    comment: Option<String>,
}

fn main() {
    // YAML ファイルが変更されたら再ビルド
    println!("cargo:rerun-if-changed=resources/tests/test-manifest.yaml");
    println!("cargo:rerun-if-changed=resources/tests");

    generate_tests();
}

fn generate_tests() {
    let manifest_path = "resources/tests/test-manifest.yaml";
    let manifest_content = fs::read_to_string(manifest_path).expect("Failed to read test-manifest.yaml");

    let manifest: TestManifest =
        serde_yaml::from_str(&manifest_content).expect("Failed to parse test-manifest.yaml");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    for test in manifest.tests {
        match test.test_type.as_str() {
            "simulation" => write_simulation_tests(&mut f, &test),
            _ => {
                panic!("Unknown test type: {}", test.test_type);
            }
        }
    }
}

fn write_simulation_tests(f: &mut fs::File, test: &TestCase) {
    let comment_line = if let Some(comment) = &test.comment {
        format!("// {}\n", comment)
    } else {
        String::new()
    };

    // resources/tests/{path}/check.json を読み込んでケース名を抽出
    let check_path = format!("resources/tests/{}/check.json", test.path);
    let check_content = fs::read_to_string(&check_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", check_path));

    #[derive(Deserialize)]
    struct CheckFile {
        cases: Vec<CaseEntry>,
    }

    #[derive(Deserialize)]
    struct CaseEntry {
        name: String,
    }

    let check: CheckFile = serde_json::from_str(&check_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}", check_path));

    for case in check.cases {
        writeln!(
            f,
            r#"{}#[test]
fn test_{}_{}_() {{
    test_simulation_case("{}", "{}")
}}
"#,
            comment_line, test.name, case.name, test.path, case.name
        )
        .unwrap();
    }
}
