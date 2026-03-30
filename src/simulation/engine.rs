use std::collections::HashMap;

use crate::base::{Rect, SimulationError};
use crate::circuit::{Circuit, InputComponent, Output, Pos};
use crate::simulation::wire_state::WireSimState;

/// `Simulator::step()` の戻り値。
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

/// シミュレーションエンジンの公開インターフェース。
pub trait Simulator {
    /// 1 セル分だけ進める。中断ポイント。
    fn step(&mut self) -> StepResult;

    /// 直近で完了した tick のテスター検証を行い、不一致を返す。
    fn verify_testers(&self) -> Vec<TesterResult>;

    /// 回路定義を取得する。
    fn circuit(&self) -> &Circuit;

    /// 指定セルの値を更新する。
    fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError>;

    /// 現在の出力キャッシュを返す。
    fn last_output(&self) -> &TickOutput;

    /// 現在の状態から出力キャッシュを再構築する。
    fn replay_tick(&mut self);

    /// 完了した tick 数を返す。
    fn current_tick(&self) -> u64;

    /// 現在 tick 内で次に処理されるセルを返す。
    fn current_cell(&self) -> Option<Pos>;

    /// 出力形式を変更する。
    fn set_output_format(&mut self, output_format: OutputFormat);

    /// tick 内の更新処理中かどうかを返す。
    fn is_updating(&self) -> bool;

    /// 1 tick 完了まで進める。
    /// 呼び出し前後で更新完了状態が保証される。
    fn tick(&mut self) {
        while self.step() != StepResult::TickComplete {}
    }

    /// 指定 tick 数だけ進める。
    /// 呼び出し前後で更新完了状態が保証される。
    fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    /// 指定 tick 数だけ進め、各 tick の状態を収集して返す。
    /// 呼び出し前後で更新完了状態が保証される。
    fn run_with_snapshots(&mut self, ticks: u64) -> Vec<TickOutput> {
        let mut snapshots = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            self.tick();
            snapshots.push(self.last_output().clone());
        }
        snapshots
    }

    /// 指定 tick 数だけ進め、各 tick のテスター検証結果を収集して返す。
    /// 呼び出し前後で更新完了状態が保証される。
    fn run_with_verification(&mut self, ticks: u64) -> Vec<TesterResult> {
        let mut mismatches = Vec::new();
        for _ in 0..ticks {
            self.tick();
            mismatches.extend(self.verify_testers());
        }
        mismatches
    }
}

/// 遅延ワイヤベースの中断可能シミュレーションエンジン。
///
/// tick 内の途中状態（`cell_index > 0`）を「更新中」と呼ぶ。
/// 更新中は一部のフィールドが不完全な値を持つため、
/// 状態の読み取りには注意が必要。
#[derive(Debug, Clone)]
pub struct SimulatorSimple {
    /// 回路定義。不変。更新中でも常に有効。
    circuit: Circuit,
    /// 遅延ワイヤおよび入力なしセルの前 tick 値。
    /// 更新中: 前 tick の値を保持（現 tick の step() で読み取られる）。
    /// complete_tick() で現 tick の値に更新される。
    wire_state: WireSimState,
    /// 全セルの現在値。sorted_cells() と同じ順序でインデックスされる。
    /// 更新中: インデックス `< cell_index` のセルは現 tick の計算済み値、
    ///         `>= cell_index` のセルは前 tick の値のまま。
    /// complete_tick() 完了後は全セルが現 tick の値を持つ。
    cell_values: Vec<bool>,
    /// Pos → cell_values インデックスの逆引きマップ。不変。更新中でも常に有効。
    cell_pos_to_index: HashMap<Pos, usize>,
    /// 完了した tick 数（0-based の次 tick 番号）。
    /// 更新中: 現在処理中の tick の番号を保持。complete_tick() でインクリメントされる。
    tick: u64,
    /// 現在 tick 内で次に処理するセルのインデックス。
    /// 0 = tick 間の待機状態（更新完了）。
    /// 1 以上 = tick 内の更新中。
    /// complete_tick() で 0 にリセットされる。
    cell_index: usize,
    /// 直近の完了 tick の出力キャッシュ。
    /// 更新中: 前回の complete_tick() 時点の出力を保持しており、現 tick の途中経過は反映されない。
    /// complete_tick() で現 tick の最終値から再構築される。
    last_output: TickOutput,
    /// 出力形式の設定。不変（set_output_format で変更可能）。更新中でも常に有効。
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
            wire_state: WireSimState::from_circuit(&circuit),
            circuit,
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
        for (wire_index, wire) in self.circuit.wires().iter().enumerate() {
            if wire.dst < wire.src {
                let src_idx = self.cell_pos_to_index[&wire.src];
                self.wire_state
                    .update_wire(wire_index, self.cell_values[src_idx]);
            }
        }

        for (cell_idx, &pos) in self.circuit.sorted_cells().iter().enumerate() {
            if self.circuit.incoming_indices(pos).is_empty()
                && !self
                    .circuit
                    .inputs()
                    .iter()
                    .any(|input| input.target() == pos)
            {
                self.wire_state
                    .update_cell(cell_idx, self.cell_values[cell_idx]);
            }
        }

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
    ///
    /// `cell_values`、`wire_state`、`last_output` を即時更新する。
    /// 更新中に呼び出した場合、注入した値がその後の step() で上書きされる可能性がある。
    /// 更新完了状態で呼び出すことを推奨。
    fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
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

        // Bug fix: callers switched from direct cell accessors to last_output(), so injected
        // values must immediately refresh the cache instead of waiting for the next tick.
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

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;
