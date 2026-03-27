# テスト用 Input/Output コンポーネント (Generator / Tester)

テスト用の入力コンポーネント（Generator）と出力コンポーネント（Tester）を設計し、回路定義の中でテスト対象の期待値を表現できるようにする。

作成日: 2026-03-27
ステータス: 完了

## 進捗

- 2026-03-27: Input/Output trait 体系 (`InputComponent` / `OutputComponent`) を実装
- 2026-03-27: `Input` / `Output` enum を追加し、`Generator` / `Tester` を統合
- 2026-03-27: `Circuit` を `inputs` / `outputs` 保持へ拡張、バリデーションを Input/Output 用に更新
- 2026-03-27: JSON を `input` / `output` + `type` 形式へ対応（`generators` / `testers` は後方互換で受理）
- 2026-03-27: `Simulator` に `verify_testers()` / `run_with_verification()` を追加
- 2026-03-27: テスト追加（unit + feature resources）と `cargo test` 全件成功

## 背景・動機

### コンポーネントの概念

LGCELL2 では Wire のみが回路上のコンポーネントとして存在するが、今後以下のカテゴリのコンポーネントを導入予定:

- **Input**: 回路外部から入力を発生させるコンポーネント（ボタン、周期入力、ファイル入力など）
  - 関連付けられるセルを dst とする Wire を接続できない（入力は外部からのみ）
- **Output**: 回路のセル状態を外部に可視化・出力するコンポーネント（LED、外部端子など）

### 現状と課題

- **Generator**（テスト用 Input）は既に実装済み。tick ごとにパターンで値を注入する。
- **Tester**（テスト用 Output）は未実装。現在はテスト期待値が `check.json` の `expected` フィールドに分散しており、回路定義とテスト検証が分離している。
- Tester を導入すると、回路定義内で入力パターンと期待出力パターンを一体的に定義でき、テストの自己完結性が向上する。

## 設計・方針

### 全体像: Input / Output コンポーネント分類

```
Input コンポーネント（回路外 → 回路へ値を注入）
├── Generator  [テスト用, 実装済み] — tick ごとのパターンで値を注入
├── Button     [将来] — クリック中 1 / 瞬間パルス / トグル
└── Clock      [将来] — 周期的な入力（tick 基準 / 時間基準）

Output コンポーネント（回路 → 外部へ値を出力）
├── Tester     [テスト用, 未実装] — tick ごとの期待パターンで検証
├── Light      [将来] — セル状態に応じて点灯・消灯
└── Terminal   [将来] — 外部ツール接続端子
```

今回は **Tester** の新規設計と、既存 Generator の Input/Output trait 体系への統合を行う。

### Trait 設計

Input / Output コンポーネントの共通インターフェースを trait で定義し、各コンポーネントがそれを実装する。Circuit での格納は enum ディスパッチを使用する。

```rust
/// Input コンポーネント共通 trait。回路外から値を注入する。
pub trait InputComponent {
    /// 対象セルの座標を返す。
    fn target(&self) -> Pos;

    /// 指定 tick における出力値を返す。
    fn value_at(&self, tick: u64) -> bool;
}

/// Output コンポーネント共通 trait。回路のセル値を観測する。
pub trait OutputComponent {
    /// 対象セルの座標を返す。
    fn target(&self) -> Pos;
}
```

### Input / Output enum

Circuit 内で格納するために、Clone + Debug を満たす enum を定義する。enum は対応する trait を委譲実装する。

```rust
/// Input コンポーネントの enum。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Generator(Generator),
}

impl InputComponent for Input {
    fn target(&self) -> Pos {
        match self {
            Input::Generator(g) => g.target(),
        }
    }
    fn value_at(&self, tick: u64) -> bool {
        match self {
            Input::Generator(g) => g.value_at(tick),
        }
    }
}

/// Output コンポーネントの enum。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Tester(Tester),
}

impl OutputComponent for Output {
    fn target(&self) -> Pos {
        match self {
            Output::Tester(t) => t.target(),
        }
    }
}
```

将来 Button / Clock / Light 等が追加される場合はバリアントを追加するだけで済む。

### Generator（既存 → InputComponent 実装へ移行）

```rust
pub struct Generator {
    target: Pos,
    pattern: Vec<bool>,
    is_loop: bool,
}

impl InputComponent for Generator {
    fn target(&self) -> Pos { self.target }
    fn value_at(&self, tick: u64) -> bool { /* 既存ロジック */ }
}
```

- `target`: 値を注入するセルの座標
- `pattern`: tick ごとの出力値（`"101"` → `[true, false, true]`）
- `is_loop`: パターンを繰り返すかどうか。false の場合、パターン末尾の値を保持

**制約（既存）:**
- target セルに incoming wire があってはならない（`CircuitError::InputTargetHasIncomingWires`）
- 同一 target の Input コンポーネントは複数定義不可（`CircuitError::DuplicateInputTarget`）
- pattern は空であってはならない（`CircuitError::EmptyGeneratorPattern`）

### Tester（新規・OutputComponent 実装）

```rust
/// tick ごとに期待パターンでセル値を検証するテスター。
pub struct Tester {
    target: Pos,
    expected: Vec<Option<bool>>,
    is_loop: bool,
}

impl OutputComponent for Tester {
    fn target(&self) -> Pos { self.target }
}
```

- `target`: 観測対象セルの座標
- `expected`: tick ごとの期待値。`None` は「その tick では検証しない（don't care）」
- `is_loop`: パターンを繰り返すかどうか。false の場合、パターン長を超えた tick では検証しない

#### パターン文字列のフォーマット

JSON 上は文字列として定義し、3種類の文字を使用:

| 文字 | 意味 | `Option<bool>` への変換 |
|------|------|------------------------|
| `1`  | true を期待 | `Some(true)` |
| `0`  | false を期待 | `Some(false)` |
| `x`  | don't care（検証しない） | `None` |

例: `"x1x0"` → `[None, Some(true), None, Some(false)]`

#### 期待値の判定ロジック

```rust
impl Tester {
    /// 指定 tick における期待値を返す。
    pub fn expected_at(&self, tick: u64) -> Option<bool> {
        let len = self.expected.len() as u64;
        if self.is_loop {
            self.expected[(tick % len) as usize]
        } else if tick < len {
            self.expected[tick as usize]
        } else {
            None // パターン外 → 検証しない
        }
    }
}
```

#### 制約

- 同一 target の Output コンポーネントは複数定義不可（`CircuitError::DuplicateOutputTarget`）
- Input の target と Output の target は重複可（観測は入力と独立）
- expected パターンは空であってはならない（`CircuitError::EmptyTesterPattern`）

### JSON フォーマット

既存の `generators` フィールドを廃止し、`input` / `output` 配列に統合する。各要素は `type` フィールドでコンポーネント種別を区別する。

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
  ],
  "input": [
    { "type": "generator", "target": [0, 0], "pattern": "10", "loop": true }
  ],
  "output": [
    { "type": "tester", "target": [1, 0], "expected": "x1x0", "loop": false }
  ]
}
```

#### serde によるデシリアライズ

`type` フィールドによるタグ付き enum デシリアライズを使用する:

```rust
/// Input コンポーネントの JSON モデル。
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

/// Output コンポーネントの JSON モデル。
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

/// 回路 JSON 全体。
#[derive(Debug, Deserialize)]
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
    #[serde(default)]
    pub input: Vec<InputJson>,
    #[serde(default)]
    pub output: Vec<OutputJson>,
}
```

#### 後方互換性

既存の `generators` フィールドを持つ JSON を段階的に移行するため、パーサーで `generators` も受け付ける過渡期を設ける:

```rust
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
    #[serde(default)]
    pub input: Vec<InputJson>,
    #[serde(default)]
    pub output: Vec<OutputJson>,
    /// deprecated: input に移行。両方指定された場合はマージする。
    #[serde(default)]
    pub generators: Vec<GeneratorJson>,
}
```

既存テストの `circuit.json` は新フォーマットに書き換える。

### Circuit への統合

```rust
pub struct Circuit {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    inputs: Vec<Input>,         // generators → inputs に変更
    outputs: Vec<Output>,       // 追加
    incoming: HashMap<Pos, Vec<usize>>,
    sorted_cells: Vec<Pos>,
}
```

- `with_generators` は `with_components` に改名（既存シグネチャは deprecated ラッパーで維持）
- Input の target セルは自動的に cells に追加される（Generator と同様）
- Output の target セルも自動的に cells に追加される
- Input の target セルは incoming wire を持ってはならない（既存制約を Input 全体に一般化）
- Output の target セルは incoming wire を持ってよい

### Simulator への統合

Simulator の `apply_generators` は `InputComponent` trait を使って一般化:

```rust
fn apply_inputs(&mut self) {
    for input in self.circuit.inputs() {
        let value = input.value_at(self.tick);
        self.prev_state.set(input.target(), value).expect("...");
        self.curr_state.set(input.target(), value).expect("...");
    }
}
```

Tester はシミュレーションの実行自体には影響しない（読み取り専用の観測者）。検証は Simulator に `verify_testers()` メソッドを追加して実行する:

```rust
/// テスト検証結果。
pub struct TesterResult {
    pub target: Pos,
    pub tick: u64,
    pub expected: bool,
    pub actual: bool,
}

impl Simulator {
    /// 現在の tick における全 Tester の検証を行い、不一致があれば返す。
    pub fn verify_testers(&self) -> Vec<TesterResult> { ... }
}
```

`tick()` 完了後に `verify_testers()` を呼び、不一致を収集する。

### テストフレームワークへの統合

`run_with_snapshots` に似た形で、テスト実行と Tester 検証を一体化する API を提供:

```rust
impl Simulator {
    /// 指定 tick 数だけ実行し、各 tick で Tester の検証結果を収集する。
    pub fn run_with_verification(&mut self, ticks: u64) -> Vec<TesterResult> { ... }
}
```

既存の `check.json` ベースのテストフレームワークはそのまま維持する。Tester を使ったテストは新しいテストタイプ（`type: simulation` はそのまま、`output` に tester があれば自動検証）として追加可能。

### エラー型の変更

`CircuitError` の既存バリアントをリネームし、新バリアントを追加:

```rust
pub enum CircuitError {
    // ... 既存ワイヤ系 ...

    // Input (旧 Generator 系をリネーム)
    #[error("input target {0} must not have incoming wires")]
    InputTargetHasIncomingWires(Pos),

    #[error("duplicate input target is not allowed: {0}")]
    DuplicateInputTarget(Pos),

    #[error("generator pattern must not be empty: {0}")]
    EmptyGeneratorPattern(Pos),

    // Output (新規)
    #[error("duplicate output target is not allowed: {0}")]
    DuplicateOutputTarget(Pos),

    #[error("tester expected pattern must not be empty: {0}")]
    EmptyTesterPattern(Pos),
}
```

`FormatError` に以下を追加:

```rust
pub enum FormatError {
    // ... 既存 ...
    #[error("invalid expected pattern character: '{0}' (expected '0', '1', or 'x')")]
    InvalidExpectedPatternChar(char),
}
```

## ステップ

1. **Trait 定義** (`src/circuit/component.rs`)
   - `InputComponent` trait, `OutputComponent` trait
   - `Input` enum, `Output` enum（trait の委譲実装）

2. **`Tester` 構造体の実装** (`src/circuit/tester.rs`)
   - `Tester` struct, `new()`, `target()`, `expected()`, `is_loop()`, `expected_at()`
   - `OutputComponent` 実装
   - ユニットテスト

3. **`Generator` への `InputComponent` 実装** (`src/circuit/generator.rs`)
   - `InputComponent` trait 実装追加
   - 既存メソッドはそのまま維持

4. **エラー型の変更** (`src/base/error.rs`)
   - Generator 系バリアントを Input 系にリネーム
   - Output 系バリアント追加
   - `FormatError` に 1 バリアント追加

5. **`Circuit` の変更** (`src/circuit/circuit.rs`)
   - `generators` → `inputs: Vec<Input>`, `outputs: Vec<Output>` に変更
   - `with_generators` → `with_components` に改名（互換ラッパー維持）
   - コンストラクタでの Input/Output バリデーション
   - `inputs()`, `outputs()` アクセサ追加
   - ユニットテスト

6. **JSON パーサーの変更** (`src/io/json.rs`)
   - `GeneratorJson` → `InputJson` tagged enum に変更
   - `OutputJson` tagged enum 追加
   - `CircuitJson` フィールド変更 (`input`, `output`)
   - `parse_expected_pattern()` 関数追加
   - `TryFrom<CircuitJson>` の更新
   - 後方互換: `generators` フィールドのマージ対応
   - ユニットテスト

7. **`Simulator` の変更** (`src/simulation/engine.rs`)
   - `apply_generators` → `apply_inputs`（`InputComponent` trait 使用）
   - `TesterResult` 構造体
   - `verify_testers()` メソッド
   - `run_with_verification()` メソッド
   - ユニットテスト

8. **既存テストリソースの移行** (`resources/tests/`)
   - 既存 `circuit.json` の `generators` を `input` フォーマットに書き換え

9. **テストフレームワークへの統合** (`tests/test_helpers.rs`)
   - Tester がある場合の自動検証ロジック追加
   - `check.json` の `generators` override を `input` override に変更

10. **新規テストケースの追加** (`resources/tests/`)
    - Tester を使ったシミュレーションテスト
    - Tester バリデーション用 parse_error テスト

11. **アーキテクチャドキュメントの更新** (`docs-ai/architecture/data-model.md`)
    - Input/Output trait, enum, Tester の説明を追加
