use std::collections::HashMap;

use crate::base::SimulationError;
use crate::circuit::{Circuit, InputComponent, Output, Pos};
use crate::simulation::engine::{OutputFormat, Simulator, StepResult, TesterResult, TickOutput};


/// 中断可能シミュレーションエンジン。
///
/// tick 内の途中状態（`cell_index > 0`）を「更新中」と呼ぶ。
/// 更新中は一部のフィールドが不完全な値を持つため、
/// 状態の読み取りには注意が必要。
#[derive(Debug, Clone)]
pub struct SimulatorSimple {
    /// 回路定義。不変。更新中でも常に有効。
    circuit: Circuit,
    /// 前 tick の全セル値。sorted_cells() と同じ順序でインデックスされる。
    prev_cell_values: Vec<bool>,
    /// 全セルの現在値。sorted_cells() と同じ順序でインデックスされる。
    cell_values: Vec<bool>,
    /// Pos → cell_values インデックスの逆引きマップ。不変。
    cell_pos_to_index: HashMap<Pos, usize>,
    tick: u64,
    cell_index: usize,
    last_output: TickOutput,
    output_format: OutputFormat,
}

impl SimulatorSimple {
    /// AllCell 形式でシミュレータを構築する。
    /// 構築直後は更新完了状態（`is_updating() == false`）。
    pub fn new(circuit: Circuit) -> Self {
        Self::with_output_format(circuit, OutputFormat::AllCell)
    }

    /// 出力形式を指定してシミュレータを構築する。
    /// 構築直後は更新完了状態（`is_updating() == false`）。
    pub fn with_output_format(circuit: Circuit, output_format: OutputFormat) -> Self {
        let cell_count = circuit.sorted_cells().len();
        let cell_pos_to_index = circuit
            .sorted_cells()
            .iter()
            .enumerate()
            .map(|(index, &pos)| (pos, index))
            .collect::<HashMap<_, _>>();
        let last_output = TickOutput {
            tick: 0,
            cells: match &output_format {
                OutputFormat::AllCell => circuit
                    .sorted_cells()
                    .iter()
                    .copied()
                    .map(|pos| (pos, false))
                    .collect(),
                OutputFormat::ViewPort(rects) => circuit
                    .sorted_cells()
                    .iter()
                    .copied()
                    .filter(|pos| rects.iter().any(|rect| rect.contains(*pos)))
                    .map(|pos| (pos, false))
                    .collect(),
            },
        };

        Self {
            circuit,
            prev_cell_values: vec![false; cell_count],
            cell_values: vec![false; cell_count],
            cell_pos_to_index,
            tick: 0,
            cell_index: 0,
            last_output,
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
        self.prev_cell_values.copy_from_slice(&self.cell_values);
        // Bug fix: last_output used to be rebuilt after tick increment, which made snapshot
        // numbering 1-based and disconnected from verify_testers(). Rebuilding here keeps both
        // the cache and the completed tick index aligned.
        self.last_output = self.build_output();
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
}

impl Simulator for SimulatorSimple {
    fn step(&mut self) -> StepResult {
        if self.cell_index == 0 {
            self.apply_inputs();
        }

        let cell_idx = self.cell_index;
        let cell = self.circuit.sorted_cells()[cell_idx];
        let incoming = self.circuit.incoming_indices(cell);

        if incoming.is_empty() {
            // 入力コンポーネント対象セルは apply_inputs() で既に値が設定されている。
            // それ以外の入力なしセルのみ前 tick の値を引き継ぐ。
            if !self.circuit.inputs().iter().any(|i| i.target() == cell) {
                self.cell_values[cell_idx] = self.prev_cell_values[cell_idx];
            }
        } else {
            let mut next_value = false;
            for &wire_index in incoming {
                let wire = &self.circuit.wires()[wire_index];
                let src_value = if wire.dst < wire.src {
                    let src_idx = self.cell_pos_to_index[&wire.src];
                    self.prev_cell_values[src_idx]
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

    /// 直近で完了した tick のテスター検証を行い、不一致を返す。
    ///
    /// 更新中に呼び出した場合、`cell_values` が不完全なため正しい結果が得られない。
    /// 更新完了状態（`is_updating() == false`）で呼び出すこと。
    fn verify_testers(&self) -> Vec<TesterResult> {
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

    /// 回路定義を取得する。不変であるため更新中でも安全。
    fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// 指定セルの値を更新する。
    fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
        let index = self
            .cell_pos_to_index
            .get(&pos)
            .copied()
            .ok_or(SimulationError::UnknownCell(pos))?;
        self.cell_values[index] = value;
        self.prev_cell_values[index] = value;

        self.replay_tick();

        Ok(())
    }

    /// 現在の出力キャッシュを返す。
    ///
    /// 更新中は前回 complete_tick() 時点の値を返す（現 tick の途中経過は反映されない）。
    /// 更新完了状態では直近の tick 完了時の値を返す。
    fn last_output(&self) -> &TickOutput {
        &self.last_output
    }

    /// 現在の状態から出力キャッシュを再構築する。
    ///
    /// 更新中に呼び出した場合、不完全な `cell_values` から出力が構築されるため、
    /// 一部のセルが前 tick の値のままになる。
    fn replay_tick(&mut self) {
        self.last_output = self.build_output();
    }

    /// 完了した tick 数を返す（次に実行される tick の 0-based 番号でもある）。
    ///
    /// 更新中は現在処理中の tick 番号を返す。
    /// 更新完了状態では、直近に完了した tick + 1 を返す。
    fn current_tick(&self) -> u64 {
        self.tick
    }

    /// 現在 tick 内で次に処理されるセルを返す。
    ///
    /// 更新中は次の step() で処理されるセルを返す。
    /// 更新完了状態では最初のセルを返す（次の tick の先頭）。
    fn current_cell(&self) -> Option<Pos> {
        self.circuit.sorted_cells().get(self.cell_index).copied()
    }

    /// 出力形式を変更する。即時反映したい場合は `replay_tick()` を呼ぶ。
    /// 更新中でも安全に呼び出せる。
    fn set_output_format(&mut self, output_format: OutputFormat) {
        self.output_format = output_format;
    }

    /// tick 内の更新処理中かどうかを返す。
    ///
    /// `true` の場合、`cell_values` は部分的にしか更新されておらず、
    /// `last_output` は前 tick の値のまま。
    /// `false` の場合、全フィールドが整合した状態にある。
    fn is_updating(&self) -> bool {
        self.cell_index > 0
    }
}
