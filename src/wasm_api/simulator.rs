use std::collections::BTreeSet;

use wasm_bindgen::prelude::*;

use crate::circuit::{Circuit, Generator, Pos, Wire, WireKind};
use crate::io::json::{parse_circuit_json, parse_pattern};
use crate::base::SimulationError;
use crate::simulation::{StepResult, Simulator};

use super::types::{WasmCellState, WasmCircuitInput, WasmTickResult, WasmWireKind, WasmStepRunResult};

/// JavaScript から利用可能なステートフルシミュレータ。
/// 内部に `Simulator` を保持するオパーク型。
#[wasm_bindgen]
pub struct WasmSimulator {
    simulator: Simulator,
}

#[wasm_bindgen]
impl WasmSimulator {
    /// 型付き回路データから Simulator を構築する。
    #[wasm_bindgen(constructor)]
    pub fn new(input: WasmCircuitInput) -> Result<WasmSimulator, JsError> {
        let circuit = build_circuit_from_input(input).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmSimulator {
            simulator: Simulator::new(circuit),
        })
    }

    /// JSON 文字列から Simulator を構築する（後方互換）。
    #[wasm_bindgen(js_name = "fromJson")]
    pub fn from_json(circuit_json: &str) -> Result<WasmSimulator, JsError> {
        let circuit = parse_circuit_json(circuit_json).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmSimulator {
            simulator: Simulator::new(circuit),
        })
    }

    /// 指定 tick 数を実行し、最終状態を返す。
    pub fn run(&mut self, ticks: u32) -> WasmTickResult {
        self.simulator.run(u64::from(ticks));
        self.build_tick_result()
    }

    /// 最大 max_steps セル分だけ処理を進める。
    /// tick が完了すれば `completed: true` を返す。
    /// 完了しなければ `completed: false` を返し、JS 側で
    /// `setTimeout` 後に再呼び出しすることで UI フリーズを防ぐ。
    #[wasm_bindgen(js_name = "runSteps")]
    pub fn run_steps(&mut self, max_steps: u32) -> WasmStepRunResult {
        let mut steps_executed = 0u32;
        let mut ticks_completed = 0u32;
        for _ in 0..max_steps {
            let result = self.simulator.step();
            steps_executed += 1;
            if result == StepResult::TickComplete {
                ticks_completed += 1;
                break; // tick 完了で抜けることで、呼び出し側が状態を確認する機会を提供
            }
        }
        WasmStepRunResult {
            steps_executed,
            ticks_completed,
            completed: ticks_completed > 0,
        }
    }

    /// 現在の tick 番号を返す。
    #[wasm_bindgen(js_name = "currentTick", getter)]
    pub fn current_tick(&self) -> u32 {
        // u64 → u32: Web 用途では 2^32 tick を超えることは想定しない
        self.simulator.current_tick() as u32
    }

    /// 全セルの状態を返す。
    #[wasm_bindgen(js_name = "getState")]
    pub fn get_state(&self) -> Vec<WasmCellState> {
        self.build_cell_states()
    }

    /// 指定セルの値を取得する。
    #[wasm_bindgen(js_name = "getCell")]
    pub fn get_cell(&self, x: i32, y: i32) -> Option<bool> {
        let pos = Pos::new(x, y);
        self.simulator.get_cell(pos)
    }

    /// 指定セルの値を設定する（入力注入用）。
    #[wasm_bindgen(js_name = "setCell")]
    pub fn set_cell(&mut self, x: i32, y: i32, value: bool) -> Result<(), JsError> {
        let pos = Pos::new(x, y);
        self.simulator
            .set_cell(pos, value)
            .map_err(|e: SimulationError| JsError::new(&e.to_string()))
    }

    // ---- 内部ヘルパー ----

    fn build_cell_states(&self) -> Vec<WasmCellState> {
        let state = self.simulator.cell_values();
        self.simulator
            .circuit()
            .sorted_cells()
            .iter()
            .map(|pos| WasmCellState {
                x: pos.x,
                y: pos.y,
                value: state.get(pos).copied().unwrap_or(false),
            })
            .collect()
    }

    fn build_tick_result(&self) -> WasmTickResult {
        WasmTickResult {
            tick: self.simulator.current_tick(),
            cells: self.build_cell_states(),
        }
    }
}

/// `WasmCircuitInput` から内部 `Circuit` を構築する。
fn build_circuit_from_input(input: WasmCircuitInput) -> Result<Circuit, crate::base::ParseError> {
    let mut cells = BTreeSet::new();
    let mut wires = Vec::with_capacity(input.wires.len());

    for wire_input in input.wires {
        let src = Pos::new(wire_input.src[0], wire_input.src[1]);
        let dst = Pos::new(wire_input.dst[0], wire_input.dst[1]);
        let kind = match wire_input.kind {
            WasmWireKind::Positive => WireKind::Positive,
            WasmWireKind::Negative => WireKind::Negative,
        };
        cells.insert(src);
        cells.insert(dst);
        wires.push(Wire::new(src, dst, kind));
    }

    let mut generators = Vec::with_capacity(input.generators.len());
    for gen_input in input.generators {
        let target = Pos::new(gen_input.target[0], gen_input.target[1]);
        let pattern = parse_pattern(&gen_input.pattern)
            .map_err(crate::base::ParseError::from)?;
        generators.push(Generator::new(target, pattern, gen_input.is_loop));
    }

    Circuit::with_generators(cells, wires, generators).map_err(crate::base::ParseError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_api::types::{WasmWireInput, WasmWireKind};

    fn make_simple_input() -> WasmCircuitInput {
        WasmCircuitInput {
            wires: vec![WasmWireInput {
                src: [0, 0],
                dst: [1, 0],
                kind: WasmWireKind::Positive,
            }],
            generators: vec![],
        }
    }

    #[test]
    fn new_creates_simulator_from_typed_input() {
        let input = make_simple_input();
        let result = WasmSimulator::new(input);
        assert!(result.is_ok());
    }

    #[test]
    fn from_json_creates_simulator() {
        let json = r#"{"wires":[{"src":[0,0],"dst":[1,0],"kind":"positive"}]}"#;
        let result = WasmSimulator::from_json(json);
        assert!(result.is_ok());
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn from_json_rejects_invalid_json() {
        let result = WasmSimulator::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn run_returns_tick_result() {
        let input = make_simple_input();
        let mut sim = WasmSimulator::new(input).unwrap();
        let result = sim.run(3);
        assert_eq!(result.tick, 3);
        assert!(!result.cells.is_empty());
    }

    #[test]
    fn run_steps_completes_a_tick() {
        let input = make_simple_input();
        let mut sim = WasmSimulator::new(input).unwrap();
        // 十分なステップ数を与えれば 1 tick が完了する
        let result = sim.run_steps(1000);
        assert!(result.completed);
        assert_eq!(result.ticks_completed, 1);
        assert!(result.steps_executed > 0);
    }

    #[test]
    fn run_steps_with_zero_max_returns_no_progress() {
        let input = make_simple_input();
        let mut sim = WasmSimulator::new(input).unwrap();
        let result = sim.run_steps(0);
        assert!(!result.completed);
        assert_eq!(result.steps_executed, 0);
        assert_eq!(result.ticks_completed, 0);
    }

    #[test]
    fn current_tick_increments_after_run() {
        let input = make_simple_input();
        let mut sim = WasmSimulator::new(input).unwrap();
        assert_eq!(sim.current_tick(), 0);
        sim.run(2);
        assert_eq!(sim.current_tick(), 2);
    }

    #[test]
    fn get_state_returns_all_cells() {
        let input = make_simple_input();
        let sim = WasmSimulator::new(input).unwrap();
        let state = sim.get_state();
        // ワイヤの src/dst から 2 セル
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn get_cell_returns_value() {
        let input = make_simple_input();
        let sim = WasmSimulator::new(input).unwrap();
        assert_eq!(sim.get_cell(0, 0), Some(false));
        assert_eq!(sim.get_cell(1, 0), Some(false));
    }

    #[test]
    fn get_cell_returns_none_for_unknown() {
        let input = make_simple_input();
        let sim = WasmSimulator::new(input).unwrap();
        assert_eq!(sim.get_cell(99, 99), None);
    }

    #[test]
    fn set_cell_updates_value() {
        let input = make_simple_input();
        let mut sim = WasmSimulator::new(input).unwrap();
        sim.set_cell(0, 0, true).unwrap();
        assert_eq!(sim.get_cell(0, 0), Some(true));
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn set_cell_fails_for_unknown_cell() {
        let input = make_simple_input();
        let mut sim = WasmSimulator::new(input).unwrap();
        let result = sim.set_cell(99, 99, true);
        assert!(result.is_err());
    }
}
