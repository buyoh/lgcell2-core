use std::collections::{BTreeMap, HashMap};

use crate::circuit::{Pos, Wire};

/// 回路の構造定義。構築後は不変。
#[derive(Debug, Clone)]
pub struct Circuit {
    /// 全セルの初期値。BTreeMap により (x, y) 順でソート済み。
    cells: BTreeMap<Pos, bool>,
    /// 全ワイヤ。
    wires: Vec<Wire>,
    /// dst でグループ化したワイヤインデックス（事前計算）。
    incoming: HashMap<Pos, Vec<usize>>,
    /// ソート済みセル座標リスト（事前計算）。
    sorted_cells: Vec<Pos>,
}

impl Circuit {
    /// セル定義とワイヤ定義から回路を構築する。
    pub fn new(cells: BTreeMap<Pos, bool>, wires: Vec<Wire>) -> Result<Self, String> {
        for wire in &wires {
            if wire.src == wire.dst {
                return Err(format!(
                    "self-loop wire is not allowed: src=({}, {}), dst=({}, {})",
                    wire.src.x, wire.src.y, wire.dst.x, wire.dst.y
                ));
            }

            if !cells.contains_key(&wire.src) {
                return Err(format!(
                    "wire src does not exist in cells: ({}, {})",
                    wire.src.x, wire.src.y
                ));
            }

            if !cells.contains_key(&wire.dst) {
                return Err(format!(
                    "wire dst does not exist in cells: ({}, {})",
                    wire.dst.x, wire.dst.y
                ));
            }
        }

        let mut incoming: HashMap<Pos, Vec<usize>> = HashMap::new();
        for (idx, wire) in wires.iter().enumerate() {
            incoming.entry(wire.dst).or_default().push(idx);
        }

        let sorted_cells = cells.keys().copied().collect::<Vec<_>>();

        Ok(Self {
            cells,
            wires,
            incoming,
            sorted_cells,
        })
    }

    /// 全セルの初期値を返す。
    pub fn cells(&self) -> &BTreeMap<Pos, bool> {
        &self.cells
    }

    /// 全ワイヤを返す。
    pub fn wires(&self) -> &[Wire] {
        &self.wires
    }

    /// 伝搬順にソート済みのセル一覧を返す。
    pub fn sorted_cells(&self) -> &[Pos] {
        &self.sorted_cells
    }

    /// 指定セルに入るワイヤインデックス一覧を返す。
    pub fn incoming_indices(&self, dst: Pos) -> &[usize] {
        self.incoming
            .get(&dst)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// 指定セルの初期値を返す。
    pub fn initial_value(&self, pos: Pos) -> Option<bool> {
        self.cells.get(&pos).copied()
    }
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod circuit_tests;
