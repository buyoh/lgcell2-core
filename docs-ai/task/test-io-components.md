# テスト用 Input/Output コンポーネント (Generator / Tester)

テスト用の入力コンポーネント（Generator）と出力コンポーネント（Tester）を設計し、回路定義の中でテスト対象の期待値を表現できるようにする。

作成日: 2026-03-27
ステータス: 設計完了（未実装）

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

今回は **Tester** の設計・実装のみを行う。Generator は既存実装をそのまま活用する。

### Generator（既存・変更なし）

```rust
pub struct Generator {
    target: Pos,
    pattern: Vec<bool>,
    is_loop: bool,
}
```

- `target`: 値を注入するセルの座標
- `pattern`: tick ごとの出力値（`"101"` → `[true, false, true]`）
- `is_loop`: パターンを繰り返すかどうか。false の場合、パターン末尾の値を保持

**制約（既存）:**
- target セルに incoming wire があってはならない（`CircuitError::GeneratorTargetHasIncomingWires`）
- 同一 target の Generator は複数定義不可（`CircuitError::DuplicateGeneratorTarget`）
- pattern は空であってはならない（`CircuitError::EmptyGeneratorPattern`）

### Tester（新規）

```rust
/// tick ごとに期待パターンでセル値を検証するテスター。
pub struct Tester {
    target: Pos,
    expected: Vec<Option<bool>>,
    is_loop: bool,
}
```

- `target`: 観測対象セルの座標
- `expected`: tick ごとの期待値。`None` は「そのtickでは検証しない（don't care）」
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

- 同一 target の Tester は複数定義不可（`CircuitError::DuplicateTesterTarget`）
- Generator の target と Tester の target は重複不可（`CircuitError::TesterTargetIsGeneratorTarget`）
- expected パターンは空であってはならない（`CircuitError::EmptyTesterPattern`）

### JSON フォーマット

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
  ],
  "generators": [
    { "target": [0, 0], "pattern": "10", "loop": true }
  ],
  "testers": [
    { "target": [1, 0], "expected": "x1x0", "loop": false }
  ]
}
```

### Circuit への統合

```rust
pub struct Circuit {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    generators: Vec<Generator>,
    testers: Vec<Tester>,       // 追加
    incoming: HashMap<Pos, Vec<usize>>,
    sorted_cells: Vec<Pos>,
}
```

- `with_generators` を拡張するか、新たに `with_components` メソッドを追加
- Tester の target セルは自動的に cells に追加される（Generator と同様）
- ただし Tester の target セルは incoming wire を持ってよい（Generator と異なる）

### Simulator への統合

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

既存の `check.json` ベースのテストフレームワークはそのまま維持する。Tester を使ったテストは新しいテストタイプ（`type: simulation` はそのまま、`testers` が `circuit.json` にあれば自動検証）として追加可能。

### エラー型の追加

`CircuitError` に以下を追加:

```rust
pub enum CircuitError {
    // ... 既存 ...
    #[error("duplicate tester target is not allowed: {0}")]
    DuplicateTesterTarget(Pos),

    #[error("tester target {0} must not be a generator target")]
    TesterTargetIsGeneratorTarget(Pos),

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

1. **`Tester` 構造体の実装** (`src/circuit/tester.rs`)
   - `Tester` struct, `new()`, `target()`, `expected()`, `is_loop()`, `expected_at()`
   - ユニットテスト

2. **エラー型の追加** (`src/base/error.rs`)
   - `CircuitError` に 3 バリアント追加
   - `FormatError` に 1 バリアント追加

3. **`Circuit` への統合** (`src/circuit/circuit.rs`)
   - `testers` フィールド追加
   - コンストラクタでの Tester バリデーション
   - `testers()` アクセサ追加
   - ユニットテスト

4. **JSON パーサーへの統合** (`src/io/json.rs`)
   - `TesterJson` モデル追加
   - `CircuitJson.testers` フィールド追加
   - `parse_expected_pattern()` 関数追加
   - `TryFrom<CircuitJson>` の拡張
   - ユニットテスト

5. **`Simulator` への検証メソッド追加** (`src/simulation/engine.rs`)
   - `TesterResult` 構造体
   - `verify_testers()` メソッド
   - `run_with_verification()` メソッド
   - ユニットテスト

6. **テストフレームワークへの統合** (`tests/test_helpers.rs`)
   - Tester がある場合の自動検証ロジック追加

7. **テストケースの追加** (`resources/tests/`)
   - Tester を使ったシミュレーションテスト
   - Tester バリデーション用 parse_error テスト

8. **アーキテクチャドキュメントの更新** (`docs-ai/architecture/data-model.md`)
   - Tester の説明を追加
