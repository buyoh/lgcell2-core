use test_helpers::{test_simulation_case, test_validation_case};

mod test_helpers;

// build.rs でジェネレートされたテスト関数を include
include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
