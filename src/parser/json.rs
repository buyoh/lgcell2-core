use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::base::{FormatError, ParseError};
use crate::circuit::{
    Circuit, CircuitBuilder, Generator, Input, Output, Pos, ResolvedModule, Tester, WireKind,
};
use crate::simulation::{Simulator, SimulatorSimple};

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
    #[serde(default)]
    pub modules: Vec<ModuleJson>,
    #[serde(default)]
    pub subs: HashMap<String, SubCircuitJson>,
}

/// モジュールインスタンスの JSON モデル。
#[derive(Debug, Deserialize)]
pub struct ModuleJson {
    #[serde(rename = "type")]
    pub module_type: String,
    pub sub_circuit: Option<String>,
    pub input: Vec<[i32; 2]>,
    pub output: Vec<[i32; 2]>,
}

/// サブ回路定義の JSON モデル。
#[derive(Debug, Deserialize)]
pub struct SubCircuitJson {
    pub wires: Vec<WireJson>,
    pub sub_input: Vec<[i32; 2]>,
    pub sub_output: Vec<[i32; 2]>,
    #[serde(default)]
    pub modules: Vec<ModuleJson>,
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
        // サブ回路をトポロジカルソート順に解決
        let resolved_subs = resolve_sub_circuits(&value.subs)?;

        let mut builder = CircuitBuilder::new();

        for wire in value.wires {
            let src = Pos::new(wire.src[0], wire.src[1]);
            let dst = Pos::new(wire.dst[0], wire.dst[1]);
            let kind = parse_wire_kind(&wire.kind)?;
            builder.add_wire(src, dst, kind);
        }

        for input in value.input {
            match input {
                InputJson::Generator {
                    target,
                    pattern,
                    is_loop,
                } => {
                    let target = Pos::new(target[0], target[1]);
                    let pattern = parse_pattern(&pattern)?;
                    builder.add_input(Input::Generator(Generator::new(target, pattern, is_loop)));
                }
            }
        }

        for generator in value.generators {
            let target = Pos::new(generator.target[0], generator.target[1]);
            let pattern = parse_pattern(&generator.pattern)?;
            builder.add_input(Input::Generator(Generator::new(
                target,
                pattern,
                generator.is_loop,
            )));
        }

        for output in value.output {
            match output {
                OutputJson::Tester {
                    target,
                    expected,
                    is_loop,
                } => {
                    let target = Pos::new(target[0], target[1]);
                    let expected = parse_expected_pattern(&expected)?;
                    builder.add_output(Output::Tester(Tester::new(target, expected, is_loop)));
                }
            }
        }

        for tester in value.testers {
            let target = Pos::new(tester.target[0], tester.target[1]);
            let expected = parse_expected_pattern(&tester.expected)?;
            builder.add_output(Output::Tester(Tester::new(target, expected, tester.is_loop)));
        }

        // モジュールの解決
        for module_json in &value.modules {
            let resolved = resolve_module(module_json, &resolved_subs)?;
            builder.add_module(resolved);
        }

        builder.build().map_err(ParseError::from)
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

/// 解決済みサブ回路の情報。
struct ResolvedSubCircuit {
    circuit: Circuit,
    sub_input: Vec<Pos>,
    sub_output: Vec<Pos>,
}

/// サブ回路定義をトポロジカルソート順に解決し、名前→ResolvedSubCircuit のマップを返す。
fn resolve_sub_circuits(
    subs: &HashMap<String, SubCircuitJson>,
) -> Result<HashMap<String, ResolvedSubCircuit>, ParseError> {
    if subs.is_empty() {
        return Ok(HashMap::new());
    }

    let order = topological_sort(subs)?;
    let mut resolved: HashMap<String, ResolvedSubCircuit> = HashMap::new();

    for name in &order {
        let sub_def = &subs[name];
        let sub_input: Vec<Pos> = sub_def
            .sub_input
            .iter()
            .map(|p| Pos::new(p[0], p[1]))
            .collect();
        let sub_output: Vec<Pos> = sub_def
            .sub_output
            .iter()
            .map(|p| Pos::new(p[0], p[1]))
            .collect();
        let circuit = build_sub_circuit(sub_def, &sub_input, &sub_output, &resolved)?;
        resolved.insert(
            name.clone(),
            ResolvedSubCircuit {
                circuit,
                sub_input,
                sub_output,
            },
        );
    }

    Ok(resolved)
}

/// サブ回路の依存グラフをトポロジカルソートする。循環依存を検出する。
fn topological_sort(subs: &HashMap<String, SubCircuitJson>) -> Result<Vec<String>, ParseError> {
    // 依存グラフを構築: 各サブ回路がどのサブ回路に依存しているか（重複排除）
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for (name, sub_def) in subs {
        let mut sub_deps = Vec::new();
        for module in &sub_def.modules {
            if module.module_type == "sub" {
                if let Some(ref sub_name) = module.sub_circuit {
                    if !sub_deps.contains(&sub_name.as_str()) {
                        sub_deps.push(sub_name.as_str());
                    }
                }
            }
        }
        deps.insert(name.as_str(), sub_deps);
    }

    // Kahn のアルゴリズム
    // in_degree: そのサブ回路がまだ未解決の依存先をいくつ持つか
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for name in subs.keys() {
        in_degree.entry(name.as_str()).or_insert(0);
    }
    for (&name, dep_list) in &deps {
        // name は dep_list 内の各サブ回路に依存している
        // → name の in_degree を依存先の数だけ増やす
        *in_degree.entry(name).or_insert(0) += dep_list.len();
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&name, _)| name)
        .collect();
    queue.sort();
    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.to_string());
        // node が構築された → node に依存しているサブ回路の in_degree を減らす
        for (&name, dep_list) in &deps {
            if dep_list.contains(&node) {
                if let Some(deg) = in_degree.get_mut(name) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(name);
                        queue.sort();
                    }
                }
            }
        }
    }

    if result.len() != subs.len() {
        let remaining: Vec<String> = subs
            .keys()
            .filter(|name| !result.contains(name))
            .cloned()
            .collect();
        return Err(ParseError::CircularDependency(remaining.join(", ")));
    }

    Ok(result)
}

/// サブ回路定義から Circuit を構築する。
fn build_sub_circuit(
    sub_def: &SubCircuitJson,
    sub_input: &[Pos],
    sub_output: &[Pos],
    resolved_subs: &HashMap<String, ResolvedSubCircuit>,
) -> Result<Circuit, ParseError> {
    use std::collections::BTreeSet;

    let mut cells = BTreeSet::new();
    let mut wires = Vec::new();

    for wire_json in &sub_def.wires {
        let src = Pos::new(wire_json.src[0], wire_json.src[1]);
        let dst = Pos::new(wire_json.dst[0], wire_json.dst[1]);
        let kind = parse_wire_kind(&wire_json.kind)?;
        cells.insert(src);
        cells.insert(dst);
        wires.push(crate::circuit::Wire::new(src, dst, kind));
    }

    // sub_input / sub_output のセルを追加
    for &pos in sub_input {
        cells.insert(pos);
    }
    for &pos in sub_output {
        cells.insert(pos);
    }

    // sub_input に入力ワイヤがないか検証
    let incoming_map: HashMap<Pos, Vec<usize>> = {
        let mut map: HashMap<Pos, Vec<usize>> = HashMap::new();
        for (idx, wire) in wires.iter().enumerate() {
            map.entry(wire.dst).or_default().push(idx);
        }
        map
    };
    for &pos in sub_input {
        if incoming_map.get(&pos).map(|v| !v.is_empty()).unwrap_or(false) {
            return Err(ParseError::Circuit(
                crate::base::CircuitError::SubInputHasIncomingWires(pos),
            ));
        }
    }

    // ポート列制約検証
    Circuit::validate_port_column_public(sub_input).map_err(ParseError::Circuit)?;
    Circuit::validate_port_column_public(sub_output).map_err(ParseError::Circuit)?;

    // sub_output の x > sub_input の x
    if !sub_input.is_empty() && !sub_output.is_empty() && sub_output[0].x <= sub_input[0].x {
        return Err(ParseError::Circuit(
            crate::base::CircuitError::SubOutputBeforeSubInput,
        ));
    }

    // ネストされたモジュールの解決
    let mut modules = Vec::new();
    for module_json in &sub_def.modules {
        let resolved = resolve_module(module_json, resolved_subs)?;
        modules.push(resolved);
    }

    if modules.is_empty() {
        Circuit::with_components(cells, wires, Vec::new(), Vec::new()).map_err(ParseError::from)
    } else {
        Circuit::with_modules(cells, wires, Vec::new(), Vec::new(), modules)
            .map_err(ParseError::from)
    }
}

/// ModuleJson から ResolvedModule を構築する。
fn resolve_module(
    module_json: &ModuleJson,
    resolved_subs: &HashMap<String, ResolvedSubCircuit>,
) -> Result<ResolvedModule, ParseError> {
    if module_json.module_type != "sub" {
        return Err(ParseError::SubCircuitNotFound(
            module_json.module_type.clone(),
        ));
    }

    let sub_name = module_json
        .sub_circuit
        .as_ref()
        .ok_or_else(|| ParseError::SubCircuitNotFound("(missing sub_circuit field)".to_string()))?;

    let resolved_sub = resolved_subs
        .get(sub_name)
        .ok_or_else(|| ParseError::SubCircuitNotFound(sub_name.clone()))?;

    let input: Vec<Pos> = module_json
        .input
        .iter()
        .map(|p| Pos::new(p[0], p[1]))
        .collect();
    let output: Vec<Pos> = module_json
        .output
        .iter()
        .map(|p| Pos::new(p[0], p[1]))
        .collect();

    // カウント検証
    if input.len() != resolved_sub.sub_input.len() {
        return Err(ParseError::Circuit(
            crate::base::CircuitError::SubInputCountMismatch {
                expected: resolved_sub.sub_input.len(),
                actual: input.len(),
            },
        ));
    }
    if output.len() != resolved_sub.sub_output.len() {
        return Err(ParseError::Circuit(
            crate::base::CircuitError::SubOutputCountMismatch {
                expected: resolved_sub.sub_output.len(),
                actual: output.len(),
            },
        ));
    }

    Ok(ResolvedModule::new(
        resolved_sub.circuit.clone(),
        input,
        output,
        resolved_sub.sub_input.clone(),
        resolved_sub.sub_output.clone(),
    ))
}

/// 文字列 JSON から回路を読み込む。
pub fn parse_circuit_json(input: &str) -> Result<Circuit, ParseError> {
    let parsed = serde_json::from_str::<CircuitJson>(input)?;
    Circuit::try_from(parsed)
}

/// 回路を指定 tick だけ実行した結果を JSON モデルとして返す。
pub fn simulate_to_output_json(circuit: Circuit, ticks: u64) -> SimulationOutputJson {
    let mut simulator = SimulatorSimple::new(circuit);
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
