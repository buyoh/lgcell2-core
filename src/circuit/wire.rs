use crate::circuit::Pos;

/// ワイヤの極性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireKind {
    /// そのまま伝搬 (v)。
    Positive,
    /// 反転して伝搬 (1 - v)。
    Negative,
}

/// セル間の信号伝搬を担う有向辺。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    pub src: Pos,
    pub dst: Pos,
    pub kind: WireKind,
}

impl Wire {
    /// ワイヤを作成する。
    pub fn new(src: Pos, dst: Pos, kind: WireKind) -> Self {
        Self { src, dst, kind }
    }

    /// ワイヤ極性を適用した伝搬値を返す。
    pub fn propagate(&self, src_value: bool) -> bool {
        match self.kind {
            WireKind::Positive => src_value,
            WireKind::Negative => !src_value,
        }
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod wire_tests;
