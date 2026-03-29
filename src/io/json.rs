use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::base::{FormatError, ParseError};
use crate::circuit::{Circuit, Generator, Input, Output, Pos, Tester, Wire, WireKind};
use crate::simulation::Simulator;

/// 回路 JSON 全体を表す入力モデル。
#[derive(Debug, Deserialize)]
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
    #[serde(default)]
    pub input: Vec<InputJson>,
    #[serde(default)]
    pub output: Vec<OutputJson>,
    /// deprecated: input に移行中。パーサーで併用を許可する。
    #[serde(default)]
    pub generators: Vec<GeneratorJson>,
    /// deprecated: output に移行中。パーサーで併用を許可する。
    #[serde(default)]
    pub testers: Vec<TesterJson>,
}

/// ワイヤ入力を表す JSON モデル。
#[derive(Debug, Deserialize)]
pub struct WireJson {
    pub src: [i32; 2],
    pub dst: [i32; 2],
    pub kind: String,
}

/// ジェネレーター入力を表す JSON モデル。
#[derive(Debug, Deserialize)]
pub struct GeneratorJson {
    pub target: [i32; 2],
    pub pattern: String,
    #[serde(default, rename = "loop")]
    pub is_loop: bool,
}

/// テスター入力を表す互換 JSON モデル。
#[derive(Debug, Deserialize)]
pub struct TesterJson {
    pub target: [i32; 2],
    pub expected: String,
    #[serde(default, rename = "loop")]
    pub is_loop: bool,
}

/// Input コンポーネント入力を表す JSON モデル。
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputJson {
    Generator {
        target: [i32; 2],
        pattern: String,
        #[serde(default, rename = "loop")]
        is_loop: bool,
    },
}

/// Output コンポーネント入力を表す JSON モデル。
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputJson {
    Tester {
        target: [i32; 2],
        expected: String,
        #[serde(default, rename = "loop")]
        is_loop: bool,
    },
}

/// シミュレーション出力 JSON のルート。
#[derive(Debug, Serialize)]
pub struct SimulationOutputJson {
    pub ticks: Vec<TickStateJson>,
}

/// 単一 tick のセル状態。
#[derive(Debug, Serialize)]
pub struct TickStateJson {
    pub tick: u64,
    pub cells: BTreeMap<String, u8>,
}

impl TryFrom<CircuitJson> for Circuit {
    type Error = ParseError;

    fn try_from(value: CircuitJson) -> Result<Self, Self::Error> {
        let mut cells = BTreeSet::new();
        let mut wires = Vec::with_capacity(value.wires.len());

        for wire in value.wires {
            let src = Pos::new(wire.src[0], wire.src[1]);
            let dst = Pos::new(wire.dst[0], wire.dst[1]);
            let kind = parse_wire_kind(&wire.kind)?;

            cells.insert(src);
            cells.insert(dst);
            wires.push(Wire::new(src, dst, kind));
        }

        let mut inputs = Vec::with_capacity(value.input.len() + value.generators.len());
        for input in value.input {
            match input {
                InputJson::Generator {
                    target,
                    pattern,
                    is_loop,
                } => {
                    let target = Pos::new(target[0], target[1]);
                    let pattern = parse_pattern(&pattern)?;
                    inputs.push(Input::Generator(Generator::new(target, pattern, is_loop)));
                }
            }
        }

        for generator in value.generators {
            let target = Pos::new(generator.target[0], generator.target[1]);
            let pattern = parse_pattern(&generator.pattern)?;
            inputs.push(Input::Generator(Generator::new(
                target,
                pattern,
                generator.is_loop,
            )));
        }

        let mut outputs = Vec::with_capacity(value.output.len() + value.testers.len());
        for output in value.output {
            match output {
                OutputJson::Tester {
                    target,
                    expected,
                    is_loop,
                } => {
                    let target = Pos::new(target[0], target[1]);
                    let expected = parse_expected_pattern(&expected)?;
                    outputs.push(Output::Tester(Tester::new(target, expected, is_loop)));
                }
            }
        }

        for tester in value.testers {
            let target = Pos::new(tester.target[0], tester.target[1]);
            let expected = parse_expected_pattern(&tester.expected)?;
            outputs.push(Output::Tester(Tester::new(target, expected, tester.is_loop)));
        }

        Circuit::with_components(cells, wires, inputs, outputs).map_err(ParseError::from)
    }
}

/// ワイヤ種別文字列を WireKind に変換する。
pub fn parse_wire_kind(kind: &str) -> Result<WireKind, FormatError> {
    match kind {
        "positive" => Ok(WireKind::Positive),
        "negative" => Ok(WireKind::Negative),
        _ => Err(FormatError::InvalidWireKind(kind.to_string())),
    }
}

/// パターン文字列 (`"0"` / `"1"` の並び) を bool ベクタに変換する。
pub fn parse_pattern(pattern: &str) -> Result<Vec<bool>, FormatError> {
    pattern
        .chars()
        .map(|c| match c {
            '1' => Ok(true),
            '0' => Ok(false),
            _ => Err(FormatError::InvalidPatternChar(c)),
        })
        .collect()
}

/// 期待値パターン文字列 (`"0"` / `"1"` / `"x"`) を Option<bool> ベクタに変換する。
pub fn parse_expected_pattern(pattern: &str) -> Result<Vec<Option<bool>>, FormatError> {
    pattern
        .chars()
        .map(|c| match c {
            '1' => Ok(Some(true)),
            '0' => Ok(Some(false)),
            'x' => Ok(None),
            _ => Err(FormatError::InvalidExpectedPatternChar(c)),
        })
        .collect()
}

/// 文字列 JSON から回路を読み込む。
pub fn parse_circuit_json(input: &str) -> Result<Circuit, ParseError> {
    let parsed = serde_json::from_str::<CircuitJson>(input)?;
    Circuit::try_from(parsed)
}

/// 回路を指定 tick だけ実行した結果を JSON モデルとして返す。
pub fn simulate_to_output_json(circuit: Circuit, ticks: u64) -> SimulationOutputJson {
    let mut simulator = Simulator::new(circuit);
    let snapshots = simulator.run_with_snapshots(ticks);
    let mut results = Vec::with_capacity(snapshots.len());

    for snapshot in snapshots {
        let mut cells = BTreeMap::new();
        for (&pos, &value) in &snapshot.cells {
            cells.insert(pos_to_json_key(pos), u8::from(value));
        }

        results.push(TickStateJson {
            tick: snapshot.tick,
            cells,
        });
    }

    SimulationOutputJson { ticks: results }
}

/// Pos を JSON キー形式 (`"x,y"`) に変換する。
fn pos_to_json_key(pos: Pos) -> String {
    format!("{},{}", pos.x, pos.y)
}

/// シミュレーション結果 JSON を文字列に変換する。
pub fn output_json_to_string(output: &SimulationOutputJson) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(output)
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod json_tests;
