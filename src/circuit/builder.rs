use std::collections::BTreeSet;

use crate::base::CircuitError;
use crate::circuit::{Circuit, Input, Output, Pos, Wire, WireKind};

/// ワイヤの追加時にセルを自動推論するビルダー。
///
/// `Circuit::with_components()` を直接呼ぶ際に手動管理が必要な
/// `BTreeSet<Pos>` の構築を担い、ワイヤ端点からのセル自動登録を提供する。
pub struct CircuitBuilder {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
}

impl CircuitBuilder {
    /// 空のビルダーを作成する。
    pub fn new() -> Self {
        Self {
            cells: BTreeSet::new(),
            wires: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// ワイヤを追加し、src/dst をセルとして自動登録する。
    pub fn add_wire(&mut self, src: Pos, dst: Pos, kind: WireKind) -> &mut Self {
        self.cells.insert(src);
        self.cells.insert(dst);
        self.wires.push(Wire::new(src, dst, kind));
        self
    }

    /// Input コンポーネントを追加する。
    pub fn add_input(&mut self, input: Input) -> &mut Self {
        self.inputs.push(input);
        self
    }

    /// Output コンポーネントを追加する。
    pub fn add_output(&mut self, output: Output) -> &mut Self {
        self.outputs.push(output);
        self
    }

    /// 回路を構築する。
    pub fn build(self) -> Result<Circuit, CircuitError> {
        Circuit::with_components(self.cells, self.wires, self.inputs, self.outputs)
    }
}

impl Default for CircuitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{Generator, Tester};

    #[test]
    fn add_wire_registers_cells_automatically() {
        let mut builder = CircuitBuilder::new();
        builder.add_wire(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive);

        let circuit = builder.build().expect("circuit must be valid");
        assert!(circuit.cells().contains(&Pos::new(0, 0)));
        assert!(circuit.cells().contains(&Pos::new(1, 0)));
    }

    #[test]
    fn build_simple_circuit() {
        let mut builder = CircuitBuilder::new();
        builder.add_wire(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive);
        builder.add_wire(Pos::new(1, 0), Pos::new(2, 0), WireKind::Negative);

        let circuit = builder.build().expect("circuit must be valid");
        assert_eq!(circuit.cells().len(), 3);
        assert_eq!(circuit.wires().len(), 2);
    }

    #[test]
    fn build_with_input_output() {
        let mut builder = CircuitBuilder::new();
        builder.add_wire(Pos::new(0, 0), Pos::new(1, 0), WireKind::Positive);
        builder.add_input(Input::Generator(Generator::new(
            Pos::new(0, 0),
            vec![true, false],
            false,
        )));
        builder.add_output(Output::Tester(Tester::new(
            Pos::new(1, 0),
            vec![Some(true), Some(false)],
            false,
        )));

        let circuit = builder.build().expect("circuit must be valid");
        assert_eq!(circuit.inputs().len(), 1);
        assert_eq!(circuit.outputs().len(), 1);
    }

    #[test]
    fn build_propagates_self_loop_error() {
        let mut builder = CircuitBuilder::new();
        builder.add_wire(Pos::new(0, 0), Pos::new(0, 0), WireKind::Positive);

        let result = builder.build();
        assert!(result.is_err());
    }
}
