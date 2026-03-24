# テストケースの外部化フレームワーク

回路テストケースをソースコードから分離し、外部ファイル（JSON/YAML）として管理する仕組みを導入する。テスト関数は `build.rs` で自動生成する。

作成日: 2026-03-24
ステータス: 設計中

## 背景・動機

現在 `tests/half_adder.rs` ではテストケース（回路定義・初期値・期待値）が Rust コードに直接埋め込まれている。この方式には以下の問題がある:

- 回路定義の追加・修正に Rust の知識が必要
- テストケースの一覧性が低い
- 回路データとテストロジックが密結合
- 同一回路に対する複数のテストパターンの管理が煩雑

`local/nospace20` では YAML マニフェスト + 外部テストファイル + `build.rs` による自動生成の仕組みを採用しており、これを lgcell2-core に適用する。

## 設計・方針

### ディレクトリ構造

```
resources/
  tests/
    test-manifest.yaml          # テストケースインデックス
    simulation/                 # シミュレーションテスト
      half_adder/
        circuit.json            # 回路定義
        check.json              # テストケース（初期値・期待値）
      ...
    errors/                     # エラー系テスト（将来拡張）
      ...
```

### test-manifest.yaml（テストインデックス）

```yaml
tests:
  - name: half_adder
    type: simulation
    path: simulation/half_adder
    comment: "半加算器の真理値表テスト"
```

フィールド:
- `name`: テスト名（Rust の関数名に使用される。snake_case）
- `type`: テスト種別。初期は `simulation` のみ。将来 `parse_error` 等を追加
- `path`: `resources/tests/` からの相対パス
- `comment`: (任意) テストの説明

### circuit.json（回路定義）

既存の `CircuitJson` フォーマットをそのまま使用する:

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
    { "src": [0, 1], "dst": [1, 0], "kind": "positive" },
    { "src": [0, 0], "dst": [1, 1], "kind": "negative" },
    { "src": [0, 1], "dst": [1, 1], "kind": "negative" },
    { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
    { "src": [1, 1], "dst": [2, 0], "kind": "negative" },
    { "src": [2, 0], "dst": [3, 0], "kind": "negative" },
    { "src": [0, 0], "dst": [2, 1], "kind": "negative" },
    { "src": [0, 1], "dst": [2, 1], "kind": "negative" },
    { "src": [2, 1], "dst": [3, 1], "kind": "negative" }
  ]
}
```

### check.json（テストケース定義）

```json
{
  "ticks": 1,
  "cases": [
    {
      "name": "0_plus_0",
      "initial": { "0,0": false, "0,1": false },
      "expected": { "3,0": false, "3,1": false }
    },
    {
      "name": "0_plus_1",
      "initial": { "0,0": false, "0,1": true },
      "expected": { "3,0": true, "3,1": false }
    },
    {
      "name": "1_plus_0",
      "initial": { "0,0": true, "0,1": false },
      "expected": { "3,0": true, "3,1": false }
    },
    {
      "name": "1_plus_1",
      "initial": { "0,0": true, "0,1": true },
      "expected": { "3,0": false, "3,1": true }
    }
  ]
}
```

フィールド:
- `ticks`: シミュレーションの tick 数
- `cases`: テストケースの配列
  - `name`: ケース名（Rust 関数名の一部になる。snake_case）
  - `initial`: 初期値の設定。キー `"x,y"` → 値 `bool`。省略されたセルは `false`(デフォルト)
  - `expected`: 期待値。キー `"x,y"` → 値 `bool`。指定されたセルのみ検証

### build.rs によるテスト関数生成

nospace20 と同様に、`build.rs` でマニフェストを読み込み、`$OUT_DIR` にテストコードを生成する。

#### 生成イメージ

マニフェストの `half_adder` エントリ + check.json の各 case から、以下のようなコードを生成:

```rust
// $OUT_DIR/generated_tests.rs（build.rs が生成）
#[test]
fn test_half_adder_0_plus_0() {
    test_simulation_case("simulation/half_adder", "0_plus_0");
}

#[test]
fn test_half_adder_0_plus_1() {
    test_simulation_case("simulation/half_adder", "0_plus_1");
}

#[test]
fn test_half_adder_1_plus_0() {
    test_simulation_case("simulation/half_adder", "1_plus_0");
}

#[test]
fn test_half_adder_1_plus_1() {
    test_simulation_case("simulation/half_adder", "1_plus_1");
}
```

#### build.rs の処理フロー

1. `resources/tests/test-manifest.yaml` を読み込み
2. 各テストエントリに対して `check.json` を読み込み
3. テストタイプ (`simulation` 等) に応じたテスト関数コードを生成
4. `$OUT_DIR/generated_tests.rs` に書き出し
5. `cargo:rerun-if-changed=resources/tests` で変更検知

#### build.rs 用依存関係

`build.rs` 専用の依存は `[build-dependencies]` に追加:

```toml
[build-dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
```

### テストヘルパー関数

`tests/test_helpers.rs` にテスト実行の共通ロジックを実装する:

```rust
/// simulation タイプのテストケースを実行する。
/// test_dir: "simulation/half_adder" のような相対パス
/// case_name: "0_plus_0" のようなケース名
pub fn test_simulation_case(test_dir: &str, case_name: &str) {
    // 1. resources/tests/{test_dir}/circuit.json を読み込み
    // 2. parse_circuit_json で Circuit を構築
    // 3. resources/tests/{test_dir}/check.json を読み込み
    // 4. case_name に一致するケースを取得
    // 5. Simulator を作成し initial 値を設定
    // 6. ticks 回 tick を実行
    // 7. expected の各セル値を assert_eq で検証
}
```

### テストエントリポイント

`tests/circuit_tests.rs`（新規作成）:

```rust
mod test_helpers;

// build.rs で生成されたテスト関数を include
include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
```

### 既存テストの移行

`tests/half_adder.rs` の内容を外部ファイルに移行する:
- 回路定義 → `resources/tests/simulation/half_adder/circuit.json`
- 4パターンの真理値表 → `check.json` の `cases`
- `half_adder_is_stateless_under_alternating_inputs` は同一回路の繰り返しテストだが、check.json の cases として表現可能（同一入力を複数回定義するか、テストヘルパーに反復オプションを追加）

移行完了後、`tests/half_adder.rs` は削除する。

## ステップ

1. `resources/tests/` ディレクトリを作成し、half_adder の circuit.json / check.json を作成
2. `resources/tests/test-manifest.yaml` を作成
3. `build.rs` を実装（YAML 読み込み → テストコード生成）
4. `tests/test_helpers.rs` を実装（テスト実行共通ロジック）
5. `tests/circuit_tests.rs` を作成（generated_tests.rs の include）
6. テストが通ることを確認
7. `tests/half_adder.rs` を削除
8. `Cargo.toml` に `[build-dependencies]` を追加

## 将来の拡張

- `parse_error` タイプ: 不正な JSON 入力に対するエラーメッセージ検証
- `multi_tick` タイプ: 複数 tick にわたる状態遷移の検証
- テストケースへのタグ付けと選択実行
