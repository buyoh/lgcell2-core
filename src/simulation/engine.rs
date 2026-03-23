use crate::circuit::{Circuit, Pos};
use crate::simulation::state::SimState;

/// `Simulator::step()` の戻り値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// 1 セル処理完了。現在の tick にまだ未処理セルがある。
    Continue,
    /// 現在の tick の全セル処理完了。
    TickComplete,
}

/// 中断可能なシミュレーションエンジン。
#[derive(Debug, Clone)]
pub struct Simulator {
    circuit: Circuit,
    /// 前の tick の状態。遅延ワイヤの参照用。
    prev_state: SimState,
    /// 現在の tick で計算中の状態。
    curr_state: SimState,
    /// 現在の tick 番号 (0-origin)。
    tick: u64,
    /// 現在の tick 内で次に処理すべきセルのインデックス。
    cell_index: usize,
}

impl Simulator {
    /// 新しいシミュレータを構築する。
    pub fn new(circuit: Circuit) -> Self {
        let state = SimState::from_circuit(&circuit);
        Self {
            circuit,
            prev_state: state.clone(),
            curr_state: state,
            tick: 0,
            cell_index: 0,
        }
    }

    /// 1 セル分だけ進める。中断ポイント。
    pub fn step(&mut self) -> StepResult {
        let cell = self.circuit.sorted_cells()[self.cell_index];
        let incoming = self.circuit.incoming_indices(cell);

        if incoming.is_empty() {
            let retained = self
                .prev_state
                .get(cell)
                .expect("cell must exist in simulation state");
            self.curr_state
                .set(cell, retained)
                .expect("state update must succeed");
        } else {
            let mut next_value = false;
            for wire_index in incoming {
                let wire = &self.circuit.wires()[*wire_index];
                let src_value = if wire.dst < wire.src {
                    self.prev_state
                        .get(wire.src)
                        .expect("src must exist in previous state")
                } else {
                    self.curr_state
                        .get(wire.src)
                        .expect("src must exist in current state")
                };

                next_value = next_value || wire.propagate(src_value);
                if next_value {
                    break;
                }
            }

            self.curr_state
                .set(cell, next_value)
                .expect("state update must succeed");
        }

        self.cell_index += 1;
        if self.cell_index >= self.circuit.sorted_cells().len() {
            self.prev_state = self.curr_state.clone();
            self.cell_index = 0;
            self.tick += 1;
            StepResult::TickComplete
        } else {
            StepResult::Continue
        }
    }

    /// 1 tick 完了まで進める。
    pub fn tick(&mut self) -> &SimState {
        while self.step() != StepResult::TickComplete {}
        &self.prev_state
    }

    /// 指定 tick 数だけ進める。
    pub fn run(&mut self, ticks: u64) -> &SimState {
        for _ in 0..ticks {
            self.tick();
        }
        &self.prev_state
    }

    /// 現在の状態を取得する。
    pub fn state(&self) -> &SimState {
        &self.prev_state
    }

    /// 現在の tick 番号を取得する。
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// 現在 tick 内で処理対象のセルを返す。
    pub fn current_cell(&self) -> Option<Pos> {
        self.circuit.sorted_cells().get(self.cell_index).copied()
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;
