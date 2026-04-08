use crate::circuit::{Circuit, Pos};

/// 解決済みモジュールインスタンス。
/// サブ回路の Circuit を保持し、入出力セルの親座標⇔ローカル座標のマッピングを提供する。
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// サブ回路の回路定義（ネストされたモジュールを含む）。
    circuit: Circuit,
    /// 親座標系での入力セル位置。
    input: Vec<Pos>,
    /// 親座標系での出力セル位置。
    output: Vec<Pos>,
    /// サブ回路ローカル座標系での入力インターフェースセル。
    sub_input: Vec<Pos>,
    /// サブ回路ローカル座標系での出力インターフェースセル。
    sub_output: Vec<Pos>,
}

impl ResolvedModule {
    pub fn new(
        circuit: Circuit,
        input: Vec<Pos>,
        output: Vec<Pos>,
        sub_input: Vec<Pos>,
        sub_output: Vec<Pos>,
    ) -> Self {
        Self {
            circuit,
            input,
            output,
            sub_input,
            sub_output,
        }
    }

    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    pub fn input(&self) -> &[Pos] {
        &self.input
    }

    pub fn output(&self) -> &[Pos] {
        &self.output
    }

    pub fn sub_input(&self) -> &[Pos] {
        &self.sub_input
    }

    pub fn sub_output(&self) -> &[Pos] {
        &self.sub_output
    }
}

#[cfg(test)]
#[path = "module_tests.rs"]
mod module_tests;
