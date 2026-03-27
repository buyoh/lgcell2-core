use crate::circuit::{Generator, Pos, Tester};

/// 回路外から値を注入する Input コンポーネントの共通インターフェース。
pub trait InputComponent {
    /// 対象セルを返す。
    fn target(&self) -> Pos;

    /// 指定 tick の値を返す。
    fn value_at(&self, tick: u64) -> bool;
}

/// 回路状態を観測する Output コンポーネントの共通インターフェース。
pub trait OutputComponent {
    /// 対象セルを返す。
    fn target(&self) -> Pos;
}

/// Input コンポーネント定義。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Generator(Generator),
}

impl InputComponent for Input {
    fn target(&self) -> Pos {
        match self {
            Input::Generator(generator) => generator.target(),
        }
    }

    fn value_at(&self, tick: u64) -> bool {
        match self {
            Input::Generator(generator) => generator.value_at(tick),
        }
    }
}

/// Output コンポーネント定義。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Tester(Tester),
}

impl OutputComponent for Output {
    fn target(&self) -> Pos {
        match self {
            Output::Tester(tester) => tester.target(),
        }
    }
}