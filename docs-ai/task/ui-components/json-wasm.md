# JSON パーサ・WASM API 変更

UI コンポーネント追加に伴う JSON パーサおよび WASM API の変更を規定する。

## JSON スキーマ変更

### Input コンポーネントの拡張

既存の `InputJson` enum（`serde(tag = "type")`）に新しいバリアントを追加する:

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputJson {
    Generator {
        target: [i32; 2],
        pattern: String,
        #[serde(default, rename = "loop")]
        is_loop: bool,
    },
    ToggleBtn {
        target: [i32; 2],
    },
    PulseBtn {
        target: [i32; 2],
    },
    PushBtn {
        target: [i32; 2],
    },
}
```

JSON 例:

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
  ],
  "input": [
    { "type": "toggle_btn", "target": [0, 0] }
  ],
  "output": [
    { "type": "cell_light", "target": [1, 0] }
  ]
}
```

### Output コンポーネントの拡張

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputJson {
    Tester {
        target: [i32; 2],
        expected: String,
        #[serde(default, rename = "loop")]
        is_loop: bool,
    },
    CellLight {
        target: [i32; 2],
    },
}
```

### パーサ実装（`TryFrom<CircuitJson> for Circuit`）

`src/parser/json.rs` と `src/io/json.rs` の両方で変換処理を追加:

```rust
// Input 変換部分
for input in value.input {
    match input {
        InputJson::Generator { target, pattern, is_loop } => {
            // 既存処理
        }
        InputJson::ToggleBtn { target } => {
            let target = Pos::new(target[0], target[1]);
            inputs.push(Input::ToggleBtn(ToggleBtn::new(target)));
        }
        InputJson::PulseBtn { target } => {
            let target = Pos::new(target[0], target[1]);
            inputs.push(Input::PulseBtn(PulseBtn::new(target)));
        }
        InputJson::PushBtn { target } => {
            let target = Pos::new(target[0], target[1]);
            inputs.push(Input::PushBtn(PushBtn::new(target)));
        }
    }
}

// Output 変換部分
for output in value.output {
    match output {
        OutputJson::Tester { target, expected, is_loop } => {
            // 既存処理
        }
        OutputJson::CellLight { target } => {
            let target = Pos::new(target[0], target[1]);
            outputs.push(Output::CellLight(CellLight::new(target)));
        }
    }
}
```

## WASM API 変更

### WasmCircuitInput の拡張（`src/wasm_api/types.rs`）

新しい入力型を追加:

```rust
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmCircuitInput {
    pub wires: Vec<WasmWireInput>,
    #[serde(default)]
    pub generators: Vec<WasmGeneratorInput>,
    #[serde(default)]
    pub toggle_btns: Vec<WasmToggleBtnInput>,    // 新規
    #[serde(default)]
    pub pulse_btns: Vec<WasmPulseBtnInput>,      // 新規
    #[serde(default)]
    pub push_btns: Vec<WasmPushBtnInput>,        // 新規
    #[serde(default)]
    pub cell_lights: Vec<WasmCellLightInput>,     // 新規
    #[serde(default)]
    pub modules: Vec<WasmModuleInput>,
    #[serde(default)]
    pub sub_circuits: HashMap<String, WasmSubCircuitInput>,
}

/// トグルボタン入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmToggleBtnInput {
    pub target: [i32; 2],
}

/// パルスボタン入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmPulseBtnInput {
    pub target: [i32; 2],
}

/// プッシュボタン入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmPushBtnInput {
    pub target: [i32; 2],
}

/// セルライト入力。
#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmCellLightInput {
    pub target: [i32; 2],
}
```

### WasmSimulator の新規メソッド（`src/wasm_api/simulator.rs`）

#### インタラクティブ入力の状態設定

```rust
/// インタラクティブ入力コンポーネントの状態を設定する。
///
/// tick 境界（`apply_inputs` 呼び出し時）で適用される。
/// `isUpdating` が `true` の間に呼び出した場合、現在の tick には反映されず、
/// 次の tick の開始時に適用される。
///
/// # 引数
/// - `x`, `y`: 対象コンポーネントのセル座標
/// - `value`: 設定する値（`true` = ON, `false` = OFF）
///
/// # エラー
/// 指定座標にインタラクティブ入力コンポーネントが存在しない場合。
#[wasm_bindgen(js_name = "setInteractiveInput")]
pub fn set_interactive_input(&mut self, x: i32, y: i32, value: bool) -> Result<(), JsError> {
    let pos = Pos::new(x, y);
    self.simulator
        .set_interactive_input(pos, value)
        .map_err(|e| JsError::new(&e.to_string()))
}
```

#### コンポーネントメタデータ取得

GUI がセルの種類を判別し、適切なレンダリング（ボタン表示、ライト表示）を行うためのメタデータ取得 API:

```rust
/// インタラクティブ入力コンポーネントの一覧と現在の状態を返す。
#[wasm_bindgen(js_name = "getInteractiveInputs")]
pub fn get_interactive_inputs(&self) -> Vec<WasmInteractiveInputInfo> {
    self.simulator
        .circuit()
        .inputs()
        .iter()
        .filter_map(|input| {
            let (target, kind) = match input {
                Input::ToggleBtn(t) => (t.target(), "toggle_btn"),
                Input::PulseBtn(p) => (p.target(), "pulse_btn"),
                Input::PushBtn(p) => (p.target(), "push_btn"),
                _ => return None,
            };
            let value = self.simulator
                .get_interactive_states()
                .get(&target)
                .copied()
                .unwrap_or(false);
            Some(WasmInteractiveInputInfo {
                x: target.x,
                y: target.y,
                kind: kind.to_string(),
                value,
            })
        })
        .collect()
}

/// 出力コンポーネントの一覧を返す。
#[wasm_bindgen(js_name = "getOutputComponents")]
pub fn get_output_components(&self) -> Vec<WasmOutputComponentInfo> {
    self.simulator
        .circuit()
        .outputs()
        .iter()
        .map(|output| {
            let (target, kind) = match output {
                Output::Tester(t) => (t.target(), "tester"),
                Output::CellLight(c) => (c.target(), "cell_light"),
            };
            WasmOutputComponentInfo {
                x: target.x,
                y: target.y,
                kind: kind.to_string(),
            }
        })
        .collect()
}
```

#### 新規出力型

```rust
/// インタラクティブ入力コンポーネントの情報。
#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmInteractiveInputInfo {
    pub x: i32,
    pub y: i32,
    pub kind: String,    // "toggle_btn" | "pulse_btn" | "push_btn"
    pub value: bool,
}

/// 出力コンポーネントの情報。
#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmOutputComponentInfo {
    pub x: i32,
    pub y: i32,
    pub kind: String,    // "tester" | "cell_light"
}
```

### convert_to_circuit_json の拡張

`WasmCircuitInput` → `CircuitJson` 変換に新コンポーネントの変換を追加:

```rust
fn convert_to_circuit_json(input: WasmCircuitInput) -> CircuitJson {
    // ... 既存の wires, generators, modules 変換 ...

    // ToggleBtn → InputJson::ToggleBtn
    for btn in input.toggle_btns {
        inputs.push(InputJson::ToggleBtn { target: btn.target });
    }

    // PulseBtn → InputJson::PulseBtn
    for btn in input.pulse_btns {
        inputs.push(InputJson::PulseBtn { target: btn.target });
    }

    // PushBtn → InputJson::PushBtn
    for btn in input.push_btns {
        inputs.push(InputJson::PushBtn { target: btn.target });
    }

    // CellLight → OutputJson::CellLight
    let mut outputs = Vec::new();
    for light in input.cell_lights {
        outputs.push(OutputJson::CellLight { target: light.target });
    }

    CircuitJson {
        wires,
        input: inputs,
        output: outputs,   // ← 新規: output フィールドを追加
        modules,
        subs,
    }
}
```

## JavaScript 利用例

```javascript
// 回路の作成
const sim = new WasmSimulator({
  wires: [
    { src: [0, 0], dst: [1, 0], kind: "positive" }
  ],
  toggle_btns: [{ target: [0, 0] }],
  cell_lights: [{ target: [1, 0] }]
});

// コンポーネントメタデータの取得
const inputs = sim.getInteractiveInputs();
// → [{ x: 0, y: 0, kind: "toggle_btn", value: false }]

const outputs = sim.getOutputComponents();
// → [{ x: 1, y: 0, kind: "cell_light" }]

// ボタン操作 → tick 実行
sim.setInteractiveInput(0, 0, true);  // トグル ON
sim.run(1);

const state = sim.getCell(1, 0);  // true（ライト点灯）

// パルスボタンの例
sim.setInteractiveInput(2, 0, true);  // パルストリガー
sim.run(1);  // この tick では true
sim.run(1);  // 次の tick では自動的に false
```

## ドキュメント更新

以下のドキュメントを更新する:

- `docs/spec/circuit-json.md`: Input / Output のスキーマ表にバリアントを追加
- `docs-ai/architecture/data-model.md`: コンポーネント一覧を更新
- `docs-ai/architecture/simulation-model.md`: インタラクティブ入力の伝搬ルールを追記

## テスト方針

### JSON パーサテスト（Unit-Fake）

- `toggle_btn` / `pulse_btn` / `push_btn` / `cell_light` の JSON 解析が正常に動作すること
- 不正な `type` 値でパースエラーが発生すること
- `target` フィールドが欠けている場合にパースエラーが発生すること

### WASM API テスト（Unit-Fake）

- `WasmCircuitInput` から新コンポーネントを含む回路を構築できること
- `setInteractiveInput` で状態が変更されること
- `getInteractiveInputs` が正しいメタデータを返すこと
- `getOutputComponents` が CellLight を含むメタデータを返すこと

### テストリソース

`resources/tests/simulation/` に新コンポーネントのテスト用 JSON ファイルを追加:

- `toggle-btn-basic.json`: ToggleBtn + ワイヤの基本回路
- `pulse-btn-basic.json`: PulseBtn + ワイヤの基本回路
- `push-btn-basic.json`: PushBtn + ワイヤの基本回路
- `cell-light-basic.json`: CellLight を含む基本回路
