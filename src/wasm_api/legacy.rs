use wasm_bindgen::prelude::*;

use crate::io::json::{output_json_to_string, parse_circuit_json, simulate_to_output_json};

/// 回路 JSON をシミュレーションし、出力履歴 JSON を返す。
///
/// `ticks` は `u64`（JavaScript `BigInt`）。
#[wasm_bindgen]
pub fn simulate(circuit_json: &str, ticks: u64) -> Result<String, JsError> {
    simulate_inner(circuit_json, ticks)
}

/// 回路 JSON をシミュレーションし、出力履歴 JSON を返す。
///
/// `ticks` は `u32`（JavaScript `number`）。
#[wasm_bindgen]
pub fn simulate_n(circuit_json: &str, ticks: u32) -> Result<String, JsError> {
    simulate_inner(circuit_json, u64::from(ticks))
}

fn simulate_inner(circuit_json: &str, ticks: u64) -> Result<String, JsError> {
    let circuit =
        parse_circuit_json(circuit_json).map_err(|e| JsError::new(&e.to_string()))?;
    let output = simulate_to_output_json(circuit, ticks);
    output_json_to_string(&output).map_err(|e| JsError::new(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{simulate, simulate_n};

    #[test]
    fn simulate_returns_tick_count() {
        let circuit = r#"{"wires":[{"src":[0,0],"dst":[1,0],"kind":"positive"}]}"#;
        let output_json = simulate(circuit, 3).expect("simulation should succeed");

        assert!(output_json.contains("\"ticks\""));
    }

    #[test]
    fn simulate_n_returns_tick_count() {
        let circuit = r#"{"wires":[{"src":[0,0],"dst":[1,0],"kind":"positive"}]}"#;
        let output_json = simulate_n(circuit, 3).expect("simulation should succeed");

        assert!(output_json.contains("\"ticks\""));
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn simulate_rejects_invalid_json() {
        let result = simulate("invalid json", 1);

        assert!(result.is_err(), "invalid JSON should fail");
    }
}
