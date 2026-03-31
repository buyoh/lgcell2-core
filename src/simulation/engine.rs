use std::collections::HashMap;

use crate::base::{Rect, SimulationError};
use crate::circuit::{Circuit, Pos};

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

#[cfg(test)]
#[path = "engine_tests.rs"]
mod engine_tests;
