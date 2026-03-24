use std::collections::{BTreeSet, HashMap, HashSet};

use crate::circuit::{Generator, Pos, Wire};

/// 回路の構造定義。構築後は不変。
/// 全セルの初期値は 0 (false) 固定。
#[derive(Debug, Clone)]
pub struct Circuit {
    /// 全セルの座標。BTreeSet により (x, y) 順でソート済み。
    cells: BTreeSet<Pos>,
    /// 全ワイヤ。
    wires: Vec<Wire>,
    /// 全ジェネレーター。
    generators: Vec<Generator>,
    /// dst でグループ化したワイヤインデックス（事前計算）。
    incoming: HashMap<Pos, Vec<usize>>,
    /// ソート済みセル座標リスト（事前計算）。
    sorted_cells: Vec<Pos>,
}

impl Circuit {
    /// ジェネレーターなしで回路を構築する（既存互換）。
    pub fn new(cells: BTreeSet<Pos>, wires: Vec<Wire>) -> Result<Self, String> {
        Self::with_generators(cells, wires, Vec::new())
    }

    /// セル定義とワイヤ定義、ジェネレーター定義から回路を構築する。
    pub fn with_generators(
        mut cells: BTreeSet<Pos>,
        wires: Vec<Wire>,
        generators: Vec<Generator>,
    ) -> Result<Self, String> {
        let mut seen_pairs: HashSet<(Pos, Pos)> = HashSet::new();

        for wire in &wires {
            if wire.src == wire.dst {
                return Err(format!(
                    "self-loop wire is not allowed: src=({}, {}), dst=({}, {})",
                    wire.src.x, wire.src.y, wire.dst.x, wire.dst.y
                ));
            }

            if !cells.contains(&wire.src) {
                return Err(format!(
                    "wire src does not exist in cells: ({}, {})",
                    wire.src.x, wire.src.y
                ));
            }

            if !cells.contains(&wire.dst) {
                return Err(format!(
                    "wire dst does not exist in cells: ({}, {})",
                    wire.dst.x, wire.dst.y
                ));
            }

            if !seen_pairs.insert((wire.src, wire.dst)) {
                return Err(format!(
                    "duplicate wire is not allowed: src=({}, {}), dst=({}, {})",
                    wire.src.x, wire.src.y, wire.dst.x, wire.dst.y
                ));
            }
        }

        let mut incoming: HashMap<Pos, Vec<usize>> = HashMap::new();
        for (idx, wire) in wires.iter().enumerate() {
            incoming.entry(wire.dst).or_default().push(idx);
        }

        let mut generator_targets: HashSet<Pos> = HashSet::new();
        for generator in &generators {
            if incoming
                .get(&generator.target())
                .map(|v| !v.is_empty())
                .unwrap_or(false)
            {
                return Err(format!(
                    "generator target ({},{}) must not have incoming wires",
                    generator.target().x,
                    generator.target().y
                ));
            }

            if !generator_targets.insert(generator.target()) {
                return Err(format!(
                    "duplicate generator target is not allowed: ({},{})",
                    generator.target().x,
                    generator.target().y
                ));
            }

            if generator.pattern().is_empty() {
                return Err(format!(
                    "generator pattern must not be empty: ({},{})",
                    generator.target().x,
                    generator.target().y
                ));
            }

            cells.insert(generator.target());
        }

        let sorted_cells = cells.iter().copied().collect::<Vec<_>>();

        Ok(Self {
            cells,
            wires,
            generators,
            incoming,
            sorted_cells,
        })
    }

    /// 全セルの座標一覧を返す。
    pub fn cells(&self) -> &BTreeSet<Pos> {
        &self.cells
    }

    /// 全ワイヤを返す。
    pub fn wires(&self) -> &[Wire] {
        &self.wires
    }

    /// 全ジェネレーターを返す。
    pub fn generators(&self) -> &[Generator] {
        &self.generators
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
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod circuit_tests;
