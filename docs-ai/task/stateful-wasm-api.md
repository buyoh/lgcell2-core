# Stateful WASM API

WASM API をステートレスな関数呼び出しから、状態を保持する `WasmSimulator` クラスベースに変更する。型付きデータ構造で JS と Rust 間のデータ交換を行い、tick 単位の実行・ステップ分割実行（UI フリーズ防止）を提供する。

作成日: 2026-03-26
ステータス: 設計完了（未実装）

## 背景・動機

現在の WASM API は `simulate(circuit_json, ticks)` / `simulate_n(circuit_json, ticks)` の 2 関数のみで、以下の制約がある:

- **ステートレス**: 毎回 JSON 文字列を渡してシミュレーション全体を一括実行。途中状態の参照・操作ができない
- **UI フリーズ**: tick 数が多い場合、JavaScript のメインスレッドを長時間ブロックする
- **JSON 文字列ベース**: 入出力ともに JSON 文字列で、型安全性がなくシリアライズ/デシリアライズのオーバーヘッドがある

一方、内部の `Simulator` は既にセル単位のステップ実行（`step()`）、tick 単位実行（`tick()`）、状態の読み書き（`state()` / `state_mut()`）をサポートしている。WASM API 層でこの能力を公開する。

## 設計・方針

### 全体構成

```
wasm_api.rs (現行: 関数ベース)
    ↓ リファクタリング
wasm_api/
├── mod.rs               # モジュール定義
├── simulator.rs         # WasmSimulator クラス
├── types.rs             # WASM 公開用の型定義（入出力）
└── legacy.rs            # 後方互換の simulate / simulate_n（既存関数）
```

### 型付きデータ交換

`serde-wasm-bindgen` と `tsify-next` を使用し、Rust の型定義から TypeScript 型を自動生成する。JSON 文字列ではなく、JavaScript オブジェクトを直接やりとりする。

#### 入力型

```rust
#[derive(Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmCircuitInput {
    pub wires: Vec<WasmWireInput>,
    #[serde(default)]
    pub generators: Vec<WasmGeneratorInput>,
}

#[derive(Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmWireInput {
    pub src: [i32; 2],
    pub dst: [i32; 2],
    pub kind: WasmWireKind,
}

#[derive(Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub enum WasmWireKind {
    #[serde(rename = "positive")]
    Positive,
    #[serde(rename = "negative")]
    Negative,
}

#[derive(Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub struct WasmGeneratorInput {
    pub target: [i32; 2],
    pub pattern: String,
    #[serde(default, rename = "loop")]
    pub is_loop: bool,
}
```

生成される TypeScript 型:

```typescript
interface WasmCircuitInput {
    wires: WasmWireInput[];
    generators?: WasmGeneratorInput[];
}

interface WasmWireInput {
    src: [number, number];
    dst: [number, number];
    kind: "positive" | "negative";
}

interface WasmGeneratorInput {
    target: [number, number];
    pattern: string;
    loop?: boolean;
}
```

#### 出力型

```rust
#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmCellState {
    pub x: i32,
    pub y: i32,
    pub value: bool,
}

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmTickResult {
    pub tick: u64,
    pub cells: Vec<WasmCellState>,
}

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct WasmStepRunResult {
    pub steps_executed: u32,
    pub ticks_completed: u32,
    pub completed: bool,
}
```

生成される TypeScript 型:

```typescript
interface WasmCellState {
    x: number;
    y: number;
    value: boolean;
}

interface WasmTickResult {
    tick: number;  // u64 → number (安全範囲内で使用)
    cells: WasmCellState[];
}

interface WasmStepRunResult {
    steps_executed: number;
    ticks_completed: number;
    completed: boolean;
}
```

### WasmSimulator クラス

`#[wasm_bindgen]` で JavaScript クラスとして公開する。内部に `Simulator` を保持するオパーク型。

```rust
#[wasm_bindgen]
pub struct WasmSimulator {
    simulator: Simulator,
}

#[wasm_bindgen]
impl WasmSimulator {
    /// 型付き回路データから Simulator を構築する。
    #[wasm_bindgen(constructor)]
    pub fn new(input: WasmCircuitInput) -> Result<WasmSimulator, JsError>;

    /// JSON 文字列から Simulator を構築する（後方互換）。
    #[wasm_bindgen(js_name = "fromJson")]
    pub fn from_json(circuit_json: &str) -> Result<WasmSimulator, JsError>;

    /// 1 tick 実行し結果を返す。
    pub fn tick(&mut self) -> WasmTickResult;

    /// 指定 tick 数を実行し、最終状態を返す。
    pub fn run(&mut self, ticks: u32) -> WasmTickResult;

    /// 最大 max_steps セルぶんだけ処理を進める。
    /// tick が完了すれば completed: true を返す。
    /// 完了しなければ completed: false を返し、
    /// JS 側で setTimeout 後に再呼び出しすることで UI フリーズを防ぐ。
    #[wasm_bindgen(js_name = "runSteps")]
    pub fn run_steps(&mut self, max_steps: u32) -> WasmStepRunResult;

    /// 現在の tick 番号を返す。
    #[wasm_bindgen(js_name = "currentTick", getter)]
    pub fn current_tick(&self) -> u32;

    /// 全セルの状態を返す。
    #[wasm_bindgen(js_name = "getState")]
    pub fn get_state(&self) -> Vec<WasmCellState>;

    /// 指定セルの値を取得する。
    #[wasm_bindgen(js_name = "getCell")]
    pub fn get_cell(&self, x: i32, y: i32) -> Option<bool>;

    /// 指定セルの値を設定する（入力注入用）。
    #[wasm_bindgen(js_name = "setCell")]
    pub fn set_cell(&mut self, x: i32, y: i32, value: bool) -> Result<(), JsError>;
}
```

### setTimeout による非ブロッキング実行パターン

`run_steps(max_steps)` メソッドは、最大 `max_steps` セルぶんだけ処理を進め、tick が完了したかどうかを返す。JS 側は以下のように `setTimeout` と組み合わせて使用する:

```typescript
async function simulateNonBlocking(
    simulator: WasmSimulator,
    totalTicks: number,
    maxStepsPerChunk: number = 1000
): Promise<void> {
    let ticksDone = 0;
    while (ticksDone < totalTicks) {
        const result = simulator.runSteps(maxStepsPerChunk);
        ticksDone += result.ticks_completed;
        if (!result.completed) {
            // UI スレッドに制御を返す
            await new Promise(resolve => setTimeout(resolve, 0));
        }
    }
}
```

`run_steps` の内部実装:

```rust
pub fn run_steps(&mut self, max_steps: u32) -> WasmStepRunResult {
    let mut steps_executed = 0u32;
    let mut ticks_completed = 0u32;
    for _ in 0..max_steps {
        let result = self.simulator.step();
        steps_executed += 1;
        if result == StepResult::TickComplete {
            ticks_completed += 1;
            break;  // tick 完了で抜けることで、呼び出し側が状態を確認する機会を提供
        }
    }
    WasmStepRunResult {
        steps_executed,
        ticks_completed,
        completed: ticks_completed > 0,
    }
}
```

### 後方互換性

既存の `simulate()` / `simulate_n()` 関数は `legacy.rs` に移動し、そのまま公開を維持する。将来的に deprecated とする可能性があるが、本タスクでは削除しない。

### 依存クレートの追加

`Cargo.toml` の `[dependencies]` に以下を追加:

```toml
tsify-next = { version = "0.5", default-features = false, features = ["js"], optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
```

`wasm` feature に `tsify-next` と `serde-wasm-bindgen` を含める:

```toml
[features]
wasm = ["dep:wasm-bindgen", "dep:tsify-next", "dep:serde-wasm-bindgen"]
```

## ステップ

1. **依存クレート追加**: `Cargo.toml` に `tsify-next`, `serde-wasm-bindgen` を追加
2. **モジュール分割**: `wasm_api.rs` → `wasm_api/` ディレクトリ化
3. **型定義**: `wasm_api/types.rs` に入出力型を定義（`Tsify` derive）
4. **WasmSimulator 実装**: `wasm_api/simulator.rs` に `WasmSimulator` クラスを実装
5. **後方互換**: `wasm_api/legacy.rs` に既存関数を移動
6. **テスト**: Unit テストを追加（native テストでは内部ロジックを検証、wasm テストは既存と同様）
7. **WASM ビルド確認**: `build-wasm.sh` でビルドし、`pkg/` の `.d.ts` に新しい型定義が生成されることを確認
