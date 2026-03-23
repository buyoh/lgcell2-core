use std::collections::HashMap;

use crate::circuit::{Circuit, Pos};

/// 各セルの現在値を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimState {
    values: HashMap<Pos, bool>,
}

impl SimState {
    /// 回路のセル一覧から状態を作成する。全セルの初期値は false (0)。
    pub fn from_circuit(circuit: &Circuit) -> Self {
        let values = circuit
            .cells()
            .iter()
            .map(|pos| (*pos, false))
            .collect::<HashMap<_, _>>();

        Self { values }
    }

    /// 指定座標の値を返す。
    pub fn get(&self, pos: Pos) -> Option<bool> {
        self.values.get(&pos).copied()
    }

    /// 指定座標の値を更新する。
    pub fn set(&mut self, pos: Pos, value: bool) -> Result<(), String> {
        if let Some(entry) = self.values.get_mut(&pos) {
            *entry = value;
            Ok(())
        } else {
            Err(format!("unknown cell at ({}, {})", pos.x, pos.y))
        }
    }

    /// 内部マップを参照として返す。
    pub fn values(&self) -> &HashMap<Pos, bool> {
        &self.values
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
