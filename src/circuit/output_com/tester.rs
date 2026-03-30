use crate::circuit::{OutputComponent, Pos};

/// tick ごとの期待パターンでセル値を検証するテスター。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tester {
    target: Pos,
    expected: Vec<Option<bool>>,
    is_loop: bool,
}

impl Tester {
    /// テスターを作成する。
    pub fn new(target: Pos, expected: Vec<Option<bool>>, is_loop: bool) -> Self {
        Self {
            target,
            expected,
            is_loop,
        }
    }

    /// 観測対象セルを返す。
    pub fn target(&self) -> Pos {
        self.target
    }

    /// 期待パターンを返す。
    pub fn expected(&self) -> &[Option<bool>] {
        &self.expected
    }

    /// ループモードかどうかを返す。
    pub fn is_loop(&self) -> bool {
        self.is_loop
    }

    /// 指定 tick における期待値を返す。
    /// None は「検証しない」を表す。
    pub fn expected_at(&self, tick: u64) -> Option<bool> {
        let len = self.expected.len() as u64;
        if self.is_loop {
            self.expected[(tick % len) as usize]
        } else if tick < len {
            self.expected[tick as usize]
        } else {
            None
        }
    }
}

impl OutputComponent for Tester {
    fn target(&self) -> Pos {
        self.target()
    }
}

#[cfg(test)]
#[path = "tester_tests.rs"]
mod tester_tests;