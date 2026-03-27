use std::collections::{BTreeSet, HashMap, HashSet};

use crate::base::CircuitError;
use crate::circuit::{Generator, Input, InputComponent, Output, OutputComponent, Pos, Wire};

/// 回路の構造定義。構築後は不変。
/// 全セルの初期値は 0 (false) 固定。
#[derive(Debug, Clone)]
pub struct Circuit {
    /// 全セルの座標。BTreeSet により (x, y) 順でソート済み。
    cells: BTreeSet<Pos>,
    /// 全ワイヤ。
    wires: Vec<Wire>,
    /// 全 Input コンポーネント。
    inputs: Vec<Input>,
    /// 全 Output コンポーネント。
    outputs: Vec<Output>,
    /// dst でグループ化したワイヤインデックス（事前計算）。
    incoming: HashMap<Pos, Vec<usize>>,
    /// ソート済みセル座標リスト（事前計算）。
    sorted_cells: Vec<Pos>,
}

impl Circuit {
    /// Input/Output なしで回路を構築する（既存互換）。
    pub fn new(cells: BTreeSet<Pos>, wires: Vec<Wire>) -> Result<Self, CircuitError> {
        Self::with_components(cells, wires, Vec::new(), Vec::new())
    }

    /// セル定義とワイヤ定義、Input/Output コンポーネント定義から回路を構築する。
    pub fn with_components(
        mut cells: BTreeSet<Pos>,
        wires: Vec<Wire>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
    ) -> Result<Self, CircuitError> {
        let mut seen_pairs: HashSet<(Pos, Pos)> = HashSet::new();

        for wire in &wires {
            if wire.src == wire.dst {
                return Err(CircuitError::SelfLoop {
                    src: wire.src,
                    dst: wire.dst,
                });
            }

            if !cells.contains(&wire.src) {
                return Err(CircuitError::WireSrcNotFound(wire.src));
            }

            if !cells.contains(&wire.dst) {
                return Err(CircuitError::WireDstNotFound(wire.dst));
            }

            if !seen_pairs.insert((wire.src, wire.dst)) {
                return Err(CircuitError::DuplicateWire {
                    src: wire.src,
                    dst: wire.dst,
                });
            }
        }

        let mut incoming: HashMap<Pos, Vec<usize>> = HashMap::new();
        for (idx, wire) in wires.iter().enumerate() {
            incoming.entry(wire.dst).or_default().push(idx);
        }

        let mut input_targets: HashSet<Pos> = HashSet::new();
        for input in &inputs {
            let target = input.target();
            if incoming.get(&target).map(|v| !v.is_empty()).unwrap_or(false) {
                return Err(CircuitError::InputTargetHasIncomingWires(target));
            }

            if !input_targets.insert(target) {
                return Err(CircuitError::DuplicateInputTarget(target));
            }

            match input {
                Input::Generator(generator) => {
                    if generator.pattern().is_empty() {
                        return Err(CircuitError::EmptyGeneratorPattern(target));
                    }
                }
            }

            cells.insert(target);
        }

        let mut output_targets: HashSet<Pos> = HashSet::new();
        for output in &outputs {
            let target = output.target();
            if !output_targets.insert(target) {
                return Err(CircuitError::DuplicateOutputTarget(target));
            }

            match output {
                Output::Tester(tester) => {
                    if tester.expected().is_empty() {
                        return Err(CircuitError::EmptyTesterPattern(target));
                    }
                }
            }

            cells.insert(target);
        }

        let sorted_cells = cells.iter().copied().collect::<Vec<_>>();

        Ok(Self {
            cells,
            wires,
            inputs,
            outputs,
            incoming,
            sorted_cells,
        })
    }

    /// セル定義とワイヤ定義、ジェネレーター定義から回路を構築する。
    /// 既存互換 API として残している。
    pub fn with_generators(
        cells: BTreeSet<Pos>,
        wires: Vec<Wire>,
        generators: Vec<Generator>,
    ) -> Result<Self, CircuitError> {
        let inputs = generators
            .into_iter()
            .map(Input::Generator)
            .collect::<Vec<_>>();
        Self::with_components(cells, wires, inputs, Vec::new())
    }

    /// 全セルの座標一覧を返す。
    pub fn cells(&self) -> &BTreeSet<Pos> {
        &self.cells
    }

    /// 全ワイヤを返す。
    pub fn wires(&self) -> &[Wire] {
        &self.wires
    }

    /// 全 Input コンポーネントを返す。
    pub fn inputs(&self) -> &[Input] {
        &self.inputs
    }

    /// 全 Output コンポーネントを返す。
    pub fn outputs(&self) -> &[Output] {
        &self.outputs
    }

    /// 伝搬順にソート済みのセル一覧を返す。
    pub fn sorted_cells(&self) -> &[Pos] {
        &self.sorted_cells
    }

    /// 指定セルに入るワイヤインデックス一覧を返す。
    pub fn incoming_indices(&self, dst: Pos) -> &[usize] {
        self.incoming.get(&dst).map(Vec::as_slice).unwrap_or(&[])
    }
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod circuit_tests;
