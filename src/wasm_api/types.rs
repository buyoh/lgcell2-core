use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tsify_next::Tsify;

/// WASM への回路入力全体。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmCircuitInput {
    pub wires: Vec<WasmWireInput>,
    #[serde(default)]
    pub generators: Vec<WasmGeneratorInput>,
    #[serde(default)]
    pub modules: Vec<WasmModuleInput>,
    #[serde(default)]
    pub sub_circuits: HashMap<String, WasmSubCircuitInput>,
}

/// モジュールインスタンス入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmModuleInput {
    #[serde(rename = "type")]
    pub module_type: String,
    pub sub_circuit: Option<String>,
    pub input: Vec<[i32; 2]>,
    pub output: Vec<[i32; 2]>,
}

/// サブ回路定義入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmSubCircuitInput {
    pub wires: Vec<WasmWireInput>,
    pub sub_input: Vec<[i32; 2]>,
    pub sub_output: Vec<[i32; 2]>,
    #[serde(default)]
    pub modules: Vec<WasmModuleInput>,
}

/// ワイヤ入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmWireInput {
    pub src: [i32; 2],
    pub dst: [i32; 2],
    pub kind: WasmWireKind,
}

/// ワイヤの極性。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub enum WasmWireKind {
    #[serde(rename = "positive")]
    Positive,
    #[serde(rename = "negative")]
    Negative,
}

/// ジェネレーター入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmGeneratorInput {
    pub target: [i32; 2],
    pub pattern: String,
    #[serde(default, rename = "loop")]
    pub is_loop: bool,
}

/// 単一セルの状態。
#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmCellState {
    pub x: i32,
    pub y: i32,
    pub value: bool,
}

/// tick 実行後の結果。
#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmTickResult {
    pub tick: u64,
    pub cells: Vec<WasmCellState>,
}

/// ステップ分割実行の結果。
#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmStepRunResult {
    pub steps_executed: u32,
    pub ticks_completed: u32,
    pub completed: bool,
}
