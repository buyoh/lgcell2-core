# WASM API の対応

## 概要

WASM API（`wasm_api` モジュール）をサブ回路に対応させる。JSON パス（`from_json`）と型付きパス（`new`）の両方で、サブ回路を含む回路を構築・シミュレーション可能にする。

## 現行 API の構造

### 2 つの構築パス

1. **JSON パス**: `WasmSimulator::from_json(json)` → `parse_circuit_json()` → `Circuit`
2. **型付きパス**: `WasmSimulator::new(WasmCircuitInput)` → `build_circuit_from_input()` → `Circuit`

### 現行の型付き入力型

```rust
pub struct WasmCircuitInput {
    pub wires: Vec<WasmWireInput>,
    pub generators: Vec<WasmGeneratorInput>,
}
```

## 変更内容

### JSON パス（`from_json`）

`parse_circuit_json()` がステップ 3（JSON パース）で更新されるため、`from_json()` 自体の変更は不要。サブ回路を含む JSON がそのまま処理される。

### 型付きパス（`new`）

#### 新規型の追加（`src/wasm_api/types.rs`）

```rust
/// モジュールインスタンス入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmModuleInput {
    #[serde(rename = "type")]
    pub module_type: String,
    pub sub_circuit: Option<String>,
    pub input: Vec<[i32; 2]>,
    pub output: Vec<[i32; 2]>,
}

/// サブ回路定義入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmSubCircuitInput {
    pub wires: Vec<WasmWireInput>,
    pub sub_input: Vec<[i32; 2]>,
    pub sub_output: Vec<[i32; 2]>,
    #[serde(default)]
    pub modules: Vec<WasmModuleInput>,
}
```

#### WasmCircuitInput の拡張

```rust
pub struct WasmCircuitInput {
    pub wires: Vec<WasmWireInput>,
    #[serde(default)]
    pub generators: Vec<WasmGeneratorInput>,
    #[serde(default)]
    pub modules: Vec<WasmModuleInput>,              // 新規
    #[serde(default)]
    pub sub_circuits: HashMap<String, WasmSubCircuitInput>,  // 新規
}
```

#### `build_circuit_from_input()` の変更

`WasmCircuitInput` から `Circuit` を構築する関数に、サブ回路の解決ロジックを追加する。

処理フロー:

1. `sub_circuits` 内の定義をトポロジカルソートし、循環依存を検出
2. 依存順にサブ回路定義を `Circuit` として構築
3. 各 `WasmModuleInput` を `ResolvedModule` に変換
4. `CircuitBuilder` に modules を渡して `build()` を呼び出す

この変換ロジックは `io/json.rs` の `CircuitJson` → `Circuit` 変換と共通部分が多い。重複を避けるため、以下の 2 つの方針を検討する:

**方針 A: WasmCircuitInput → CircuitJson へ変換してから既存パスを使用**

`WasmCircuitInput` を `CircuitJson` に変換し、既存の `TryFrom<CircuitJson> for Circuit` を再利用する。変換ヘルパーを `wasm_api/simulator.rs` に追加する。

利点: サブ回路解決ロジックの重複を完全に回避
欠点: 中間変換のオーバーヘッド（軽微）

**方針 B: 共通のサブ回路解決関数を抽出**

`io/json.rs` からサブ回路解決の核となるロジックを汎用関数として抽出し、`CircuitJson` と `WasmCircuitInput` の両方から呼び出す。

利点: 直接的な変換で効率的
欠点: 抽象化レイヤの追加

**採用方針: A**

`WasmCircuitInput` は `CircuitJson` と構造的に等価であるため、`build_circuit_from_input()` が `WasmCircuitInput` → `CircuitJson` に変換し、既存の `TryFrom` を経由して `Circuit` を構築する。これによりサブ回路解決・バリデーションのロジックを一元管理できる。

```rust
fn build_circuit_from_input(input: WasmCircuitInput) -> Result<Circuit, ParseError> {
    let circuit_json = convert_to_circuit_json(input)?;
    Circuit::try_from(circuit_json).map_err(ParseError::from)
}

fn convert_to_circuit_json(input: WasmCircuitInput) -> Result<CircuitJson, ParseError> {
    // WasmWireInput → WireJson
    // WasmGeneratorInput → InputJson::Generator
    // WasmModuleInput → ModuleJson
    // WasmSubCircuitInput → SubCircuitJson
    // ...
}
```

### 既存 API メソッドへの影響

| メソッド | 変更 | 備考 |
|---|---|---|
| `run()` | なし | `Simulator::run()` が階層的シミュレーションを処理 |
| `run_steps()` | なし | `Simulator::step()` が内部的にサブ回路を評価 |
| `current_tick()` | なし | 親回路の tick カウントを返す |
| `is_updating()` | なし | 親回路の状態を返す |
| `get_state()` | なし | 親回路のセルのみ返す（サブ回路内部は不可視） |
| `get_cell()` | なし | 親回路のセルのみ参照可能 |
| `set_cell()` | なし | 親回路のセルのみ更新可能 |

サブ回路の内部セルは WASM API から直接アクセスできない。モジュール出力セルの値は親回路のセルとして `get_state()` / `get_cell()` から取得可能。

### TypeScript 型定義への影響

`tsify` の derive により、新規型の TypeScript 定義は自動生成される。`pkg/lgcell2_core.d.ts` に以下の型が追加される:

```typescript
interface WasmModuleInput {
    type: string;
    sub_circuit?: string;
    input: [number, number][];
    output: [number, number][];
}

interface WasmSubCircuitInput {
    wires: WasmWireInput[];
    sub_input: [number, number][];
    sub_output: [number, number][];
    modules?: WasmModuleInput[];
}

interface WasmCircuitInput {
    wires: WasmWireInput[];
    generators?: WasmGeneratorInput[];
    modules?: WasmModuleInput[];
    sub_circuits?: Record<string, WasmSubCircuitInput>;
}
```

### Legacy API（`simulate`, `simulate_n`）

`parse_circuit_json()` がサブ回路を処理するため、Legacy API も変更不要でサブ回路をサポートする。

## テスト

### 追加テストケース（`src/wasm_api/simulator.rs` 内の `mod tests`）

- `new` でサブ回路を含む `WasmCircuitInput` から Simulator を構築
- `from_json` でサブ回路を含む JSON から Simulator を構築
- サブ回路付き回路で `run()` を実行し、モジュール出力セルの値を `get_cell()` で検証
- `run_steps()` でサブ回路付き回路のステップ分割実行
- `get_state()` でサブ回路内部セルが含まれないことを確認
- 存在しないサブ回路名でのエラー
- 入出力カウント不一致でのエラー
