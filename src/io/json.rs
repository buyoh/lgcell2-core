use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::base::ParseError;
use crate::circuit::{Circuit, Generator, Pos, Wire, WireKind};
use crate::simulation::Simulator;

/// 回路 JSON 全体を表す入力モデル。
#[derive(Debug, Deserialize)]
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
    #[serde(default)]
    pub generators: Vec<GeneratorJson>,
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
            let kind = match wire.kind.as_str() {
                "positive" => WireKind::Positive,
                "negative" => WireKind::Negative,
                _ => return Err(ParseError::InvalidWireKind(wire.kind)),
            };

            cells.insert(src);
            cells.insert(dst);
            wires.push(Wire::new(src, dst, kind));
        }

        let mut generators = Vec::with_capacity(value.generators.len());
        for generator in value.generators {
            let target = Pos::new(generator.target[0], generator.target[1]);
            let pattern = parse_pattern(&generator.pattern)?;
            generators.push(Generator::new(target, pattern, generator.is_loop));
        }

        Circuit::with_generators(cells, wires, generators).map_err(ParseError::from)
    }
}

fn parse_pattern(pattern: &str) -> Result<Vec<bool>, ParseError> {
    pattern
        .chars()
        .map(|c| match c {
            '1' => Ok(true),
            '0' => Ok(false),
            _ => Err(ParseError::InvalidWireKind(format!(
                "invalid pattern character: '{}' (expected '0' or '1')",
                c
            ))),
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
        for (pos, value) in snapshot.cells {
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
