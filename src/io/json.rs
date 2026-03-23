use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::circuit::{Circuit, Pos, Wire, WireKind};
use crate::simulation::Simulator;

/// 回路 JSON 全体を表す入力モデル。
#[derive(Debug, Deserialize)]
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
}

/// ワイヤ入力を表す JSON モデル。
#[derive(Debug, Deserialize)]
pub struct WireJson {
    pub src: [i32; 2],
    pub dst: [i32; 2],
    pub kind: String,
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
    type Error = String;

    fn try_from(value: CircuitJson) -> Result<Self, Self::Error> {
        let mut cells = BTreeSet::new();
        let mut wires = Vec::with_capacity(value.wires.len());

        for wire in value.wires {
            let src = Pos::new(wire.src[0], wire.src[1]);
            let dst = Pos::new(wire.dst[0], wire.dst[1]);
            let kind = match wire.kind.as_str() {
                "positive" => WireKind::Positive,
                "negative" => WireKind::Negative,
                _ => return Err(format!("wire kind must be positive or negative: {}", wire.kind)),
            };

            if src == dst {
                return Err(format!(
                    "self-loop wire is not allowed: src=({}, {}), dst=({}, {})",
                    src.x, src.y, dst.x, dst.y
                ));
            }

            cells.insert(src);
            cells.insert(dst);
            wires.push(Wire::new(src, dst, kind));
        }

        Circuit::new(cells, wires)
    }
}

/// 文字列 JSON から回路を読み込む。
pub fn parse_circuit_json(input: &str) -> Result<Circuit, String> {
    let parsed = serde_json::from_str::<CircuitJson>(input).map_err(|err| err.to_string())?;
    Circuit::try_from(parsed)
}

/// 回路を指定 tick だけ実行した結果を JSON モデルとして返す。
pub fn simulate_to_output_json(circuit: Circuit, ticks: u64) -> SimulationOutputJson {
    let mut simulator = Simulator::new(circuit);
    let mut results = Vec::with_capacity(ticks as usize);

    for tick_index in 1..=ticks {
        simulator.tick();

        let mut ordered_positions = simulator.state().values().keys().copied().collect::<Vec<_>>();
        ordered_positions.sort();

        let mut cells = BTreeMap::new();
        for pos in ordered_positions {
            let value = simulator
                .state()
                .get(pos)
                .expect("position from state keys must exist");
            cells.insert(format!("{},{}", pos.x, pos.y), u8::from(value));
        }

        results.push(TickStateJson {
            tick: tick_index,
            cells,
        });
    }

    SimulationOutputJson { ticks: results }
}

/// シミュレーション結果 JSON を文字列に変換する。
pub fn output_json_to_string(output: &SimulationOutputJson) -> Result<String, String> {
    serde_json::to_string_pretty(output).map_err(|err| err.to_string())
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod json_tests;
