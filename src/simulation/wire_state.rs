use std::collections::HashMap;

use crate::circuit::{Circuit, InputComponent};

/// 遅延ワイヤベースのシミュレーション状態。
#[derive(Debug, Clone)]
pub struct WireSimState {
    delayed_values: Vec<bool>,
    wire_to_slot: HashMap<usize, usize>,
    cell_to_slot: HashMap<usize, usize>,
}

impl WireSimState {
    /// 回路定義から遅延スロットを初期化する。
    pub fn from_circuit(circuit: &Circuit) -> Self {
        let mut wire_to_slot = HashMap::new();
        let mut cell_to_slot = HashMap::new();
        let mut slot_count = 0usize;

        for (wire_index, wire) in circuit.wires().iter().enumerate() {
            if wire.dst < wire.src {
                wire_to_slot.insert(wire_index, slot_count);
                slot_count += 1;
            }
        }

        for (cell_index, pos) in circuit.sorted_cells().iter().enumerate() {
            let has_incoming = !circuit.incoming_indices(*pos).is_empty();
            let has_input = circuit.inputs().iter().any(|input| input.target() == *pos);
            if !has_incoming && !has_input {
                cell_to_slot.insert(cell_index, slot_count);
                slot_count += 1;
            }
        }

        Self {
            delayed_values: vec![false; slot_count],
            wire_to_slot,
            cell_to_slot,
        }
    }

    /// 遅延ワイヤの前 tick 値を取得する。
    pub fn get_delayed_wire(&self, wire_index: usize) -> Option<bool> {
        self.wire_to_slot
            .get(&wire_index)
            .map(|&slot| self.delayed_values[slot])
    }

    /// 入力なしセルの前 tick 値を取得する。
    pub fn get_stateless_cell(&self, cell_index: usize) -> Option<bool> {
        self.cell_to_slot
            .get(&cell_index)
            .map(|&slot| self.delayed_values[slot])
    }

    /// 遅延ワイヤの値を更新する。
    pub fn update_wire(&mut self, wire_index: usize, value: bool) {
        if let Some(&slot) = self.wire_to_slot.get(&wire_index) {
            self.delayed_values[slot] = value;
        }
    }

    /// 入力なしセルの値を更新する。
    pub fn update_cell(&mut self, cell_index: usize, value: bool) {
        if let Some(&slot) = self.cell_to_slot.get(&cell_index) {
            self.delayed_values[slot] = value;
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;