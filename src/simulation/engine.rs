use std::collections::HashMap;

use crate::base::SimulationError;
use crate::circuit::{Circuit, InputComponent, Output, Pos};
use crate::simulation::wire_state::WireSimState;

/// `WireSimulator::step()` の戻り値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// 1 セル処理完了。現在の tick にまだ未処理セルがある。
    Continue,
    /// 現在の tick の全セル処理完了。
    TickComplete,
}

/// テスター検証の不一致結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TesterResult {
    pub target: Pos,
    pub tick: u64,
    pub expected: bool,
    pub actual: bool,
}

/// 矩形領域（含む-含む）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub min: Pos,
    pub max: Pos,
}

impl Rect {
    pub fn new(min: Pos, max: Pos) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, pos: Pos) -> bool {
        pos.x >= self.min.x
            && pos.x <= self.max.x
            && pos.y >= self.min.y
            && pos.y <= self.max.y
    }
}

/// tick 完了時の出力形式。
#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// すべてのセルの状態を収集する。
    AllCell,
    /// 指定された矩形領域内のセルのみ収集する。
    ViewPort(Vec<Rect>),
}

/// 単一 tick 実行後の状態スナップショット。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickOutput {
    pub tick: u64,
    pub cells: HashMap<Pos, bool>,
}

/// 遅延ワイヤベースの中断可能シミュレーションエンジン。
#[derive(Debug, Clone)]
pub struct WireSimulator {
    circuit: Circuit,
    wire_state: WireSimState,
    cell_values: Vec<bool>,
    cell_pos_to_index: HashMap<Pos, usize>,
    tick: u64,
    cell_index: usize,
    output_format: OutputFormat,
}

impl WireSimulator {
    /// AllCell 形式でシミュレータを構築する。
    pub fn new(circuit: Circuit) -> Self {
        Self::with_output_format(circuit, OutputFormat::AllCell)
    }

    /// 出力形式を指定してシミュレータを構築する。
    pub fn with_output_format(circuit: Circuit, output_format: OutputFormat) -> Self {
        let cell_count = circuit.sorted_cells().len();
        let cell_pos_to_index = circuit
            .sorted_cells()
            .iter()
            .enumerate()
            .map(|(index, &pos)| (pos, index))
            .collect::<HashMap<_, _>>();

        Self {
            wire_state: WireSimState::from_circuit(&circuit),
            circuit,
            cell_values: vec![false; cell_count],
            cell_pos_to_index,
            tick: 0,
            cell_index: 0,
            output_format,
        }
    }

    fn apply_inputs(&mut self) {
        for input in self.circuit.inputs() {
            let value = input.value_at(self.tick);
            let target = input.target();
            if let Some(&index) = self.cell_pos_to_index.get(&target) {
                self.cell_values[index] = value;
            }
        }
    }

    fn complete_tick(&mut self) {
        for (wire_index, wire) in self.circuit.wires().iter().enumerate() {
            if wire.dst < wire.src {
                let src_idx = self.cell_pos_to_index[&wire.src];
                self.wire_state
                    .update_wire(wire_index, self.cell_values[src_idx]);
            }
        }

        for (cell_idx, &pos) in self.circuit.sorted_cells().iter().enumerate() {
            if self.circuit.incoming_indices(pos).is_empty()
                && !self.circuit.inputs().iter().any(|input| input.target() == pos)
            {
                self.wire_state.update_cell(cell_idx, self.cell_values[cell_idx]);
            }
        }

        self.cell_index = 0;
        self.tick += 1;
    }

    fn build_output(&self) -> TickOutput {
        let cells = match &self.output_format {
            OutputFormat::AllCell => self
                .circuit
                .sorted_cells()
                .iter()
                .enumerate()
                .map(|(index, &pos)| (pos, self.cell_values[index]))
                .collect(),
            OutputFormat::ViewPort(rects) => self
                .circuit
                .sorted_cells()
                .iter()
                .enumerate()
                .filter(|(_, pos)| rects.iter().any(|rect| rect.contains(**pos)))
                .map(|(index, &pos)| (pos, self.cell_values[index]))
                .collect(),
        };

        TickOutput {
            tick: self.tick,
            cells,
        }
    }

    /// 1 セル分だけ進める。中断ポイント。
    pub fn step(&mut self) -> StepResult {
        if self.cell_index == 0 {
            self.apply_inputs();
        }

        let cell_idx = self.cell_index;
        let cell = self.circuit.sorted_cells()[cell_idx];
        let incoming = self.circuit.incoming_indices(cell);

        if incoming.is_empty() {
            if let Some(value) = self.wire_state.get_stateless_cell(cell_idx) {
                self.cell_values[cell_idx] = value;
            }
        } else {
            let mut next_value = false;
            for &wire_index in incoming {
                let wire = &self.circuit.wires()[wire_index];
                let src_value = if wire.dst < wire.src {
                    self.wire_state
                        .get_delayed_wire(wire_index)
                        .expect("delayed wire must have slot")
                } else {
                    let src_idx = self.cell_pos_to_index[&wire.src];
                    self.cell_values[src_idx]
                };

                next_value = next_value || wire.propagate(src_value);
                if next_value {
                    break;
                }
            }
            self.cell_values[cell_idx] = next_value;
        }

        self.cell_index += 1;
        if self.cell_index >= self.circuit.sorted_cells().len() {
            self.complete_tick();
            StepResult::TickComplete
        } else {
            StepResult::Continue
        }
    }

    /// 1 tick 完了まで進める。
    pub fn tick(&mut self) {
        while self.step() != StepResult::TickComplete {}
    }

    /// 指定 tick 数だけ進める。
    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    /// 指定 tick 数だけ進め、各 tick の状態を収集して返す。
    pub fn run_with_snapshots(&mut self, ticks: u64) -> Vec<TickOutput> {
        let mut snapshots = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            self.tick();
            snapshots.push(self.build_output());
        }
        snapshots
    }

    /// 直近で完了した tick のテスター検証を行い、不一致を返す。
    pub fn verify_testers(&self) -> Vec<TesterResult> {
        if self.tick == 0 {
            return Vec::new();
        }

        let observed_tick = self.tick - 1;
        let mut mismatches = Vec::new();
        for output in self.circuit.outputs() {
            match output {
                Output::Tester(tester) => {
                    if let Some(expected) = tester.expected_at(observed_tick) {
                        let index = self.cell_pos_to_index[&tester.target()];
                        let actual = self.cell_values[index];
                        if actual != expected {
                            mismatches.push(TesterResult {
                                target: tester.target(),
                                tick: observed_tick,
                                expected,
                                actual,
                            });
                        }
                    }
                }
            }
        }

        mismatches
    }

    /// 指定 tick 数だけ進め、各 tick のテスター検証結果を収集して返す。
    pub fn run_with_verification(&mut self, ticks: u64) -> Vec<TesterResult> {
        let mut mismatches = Vec::new();
        for _ in 0..ticks {
            self.tick();
            mismatches.extend(self.verify_testers());
        }
        mismatches
    }

    /// 回路定義を取得する。
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// 指定セルの現在値を取得する。
    pub fn get_cell(&self, pos: Pos) -> Option<bool> {
        self.cell_pos_to_index
            .get(&pos)
            .map(|&index| self.cell_values[index])
    }

    /// 指定セルの値を更新する。
    pub fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
        let index = self
            .cell_pos_to_index
            .get(&pos)
            .copied()
            .ok_or(SimulationError::UnknownCell(pos))?;
        self.cell_values[index] = value;
        self.wire_state.update_cell(index, value);

        for (wire_index, wire) in self.circuit.wires().iter().enumerate() {
            if wire.src == pos && wire.dst < wire.src {
                self.wire_state.update_wire(wire_index, value);
            }
        }

        Ok(())
    }

    /// 現在の全セル値を返す。
    pub fn cell_values(&self) -> HashMap<Pos, bool> {
        self.circuit
            .sorted_cells()
            .iter()
            .enumerate()
            .map(|(index, &pos)| (pos, self.cell_values[index]))
            .collect()
    }

    /// 現在の tick 番号を取得する。
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// 現在 tick 内で処理対象のセルを返す。
    pub fn current_cell(&self) -> Option<Pos> {
        self.circuit.sorted_cells().get(self.cell_index).copied()
    }

    /// 出力形式を変更する。次の tick 完了から反映される。
    pub fn set_output_format(&mut self, output_format: OutputFormat) {
        self.output_format = output_format;
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;
