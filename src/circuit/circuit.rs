use std::collections::{BTreeSet, HashMap, HashSet};

use crate::base::CircuitError;
use crate::circuit::{
    Generator, Input, InputComponent, Output, OutputComponent, Pos, ResolvedModule, Wire,
};

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
    /// 全モジュールインスタンス。
    modules: Vec<ResolvedModule>,
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

    /// セル定義、ワイヤ定義、Input/Output コンポーネント定義、モジュール定義から回路を構築する。
    pub fn with_modules(
        cells: BTreeSet<Pos>,
        wires: Vec<Wire>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        modules: Vec<ResolvedModule>,
    ) -> Result<Self, CircuitError> {
        let mut circuit = Self::with_components(cells, wires, inputs, outputs)?;

        // モジュール検証
        let input_targets: HashSet<Pos> = circuit.inputs.iter().map(|i| i.target()).collect();
        let mut all_module_outputs: HashSet<Pos> = HashSet::new();

        for module in &modules {
            // ポート列制約: input
            Self::validate_port_column(module.input())?;
            // ポート列制約: output
            Self::validate_port_column(module.output())?;

            // output の x > input の x
            if !module.input().is_empty() && !module.output().is_empty() {
                let input_x = module.input()[0].x;
                let output_x = module.output()[0].x;
                if output_x <= input_x {
                    return Err(CircuitError::ModuleOutputBeforeInput);
                }
            }

            // 出力セルに入力ワイヤがないこと
            for &pos in module.output() {
                if circuit
                    .incoming
                    .get(&pos)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
                {
                    return Err(CircuitError::ModuleOutputHasIncomingWires(pos));
                }
            }

            // 出力セルが Generator ターゲットでないこと
            for &pos in module.output() {
                if input_targets.contains(&pos) {
                    return Err(CircuitError::ModuleOutputHasIncomingWires(pos));
                }
            }

            // 出力セル間の重複チェック
            for &pos in module.output() {
                if !all_module_outputs.insert(pos) {
                    return Err(CircuitError::DuplicateModuleOutput(pos));
                }
            }
        }

        // 出力セルを cells に追加
        for module in &modules {
            for &pos in module.output() {
                circuit.cells.insert(pos);
            }
            for &pos in module.input() {
                circuit.cells.insert(pos);
            }
        }
        circuit.sorted_cells = circuit.cells.iter().copied().collect();

        circuit.modules = modules;
        Ok(circuit)
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
            modules: Vec::new(),
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

    /// 全モジュールインスタンスを返す。
    pub fn modules(&self) -> &[ResolvedModule] {
        &self.modules
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

    /// ポート列制約を検証する。
    /// 全ポートが同一 x 座標で、y 座標が連続であることをチェック。
    fn validate_port_column(ports: &[Pos]) -> Result<(), CircuitError> {
        if ports.is_empty() {
            return Ok(());
        }
        let x = ports[0].x;
        for (i, port) in ports.iter().enumerate() {
            if port.x != x {
                return Err(CircuitError::InvalidPortColumn);
            }
            if port.y != ports[0].y + i as i32 {
                return Err(CircuitError::InvalidPortColumn);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod circuit_tests;
