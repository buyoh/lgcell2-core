use crate::circuit::Pos;

/// tick ごとに指定パターンで値を注入するジェネレーター。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generator {
    target: Pos,
    pattern: Vec<bool>,
    is_loop: bool,
}

impl Generator {
    /// ジェネレーターを作成する。
    pub fn new(target: Pos, pattern: Vec<bool>, is_loop: bool) -> Self {
        Self {
            target,
            pattern,
            is_loop,
        }
    }

    /// 出力先セルを返す。
    pub fn target(&self) -> Pos {
        self.target
    }

    /// 出力パターンを返す。
    pub fn pattern(&self) -> &[bool] {
        &self.pattern
    }

    /// ループモードかどうかを返す。
    pub fn is_loop(&self) -> bool {
        self.is_loop
    }

    /// 指定 tick における出力値を返す。
    pub fn value_at(&self, tick: u64) -> bool {
        let idx = tick as usize;
        if self.is_loop {
            self.pattern[idx % self.pattern.len()]
        } else {
            self.pattern[idx.min(self.pattern.len() - 1)]
        }
    }
}

#[cfg(test)]
#[path = "generator_tests.rs"]
mod generator_tests;
