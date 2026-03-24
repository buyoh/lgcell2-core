# ジェネレーター端子の設計

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

順序回路（SR ラッチ、JK フリップフロップ、カウンター等）のテストでは、tick ごとに入力信号を変化させる必要がある。現在の `initial` フィールドは tick 0 の初期値しか設定できず、複数 tick にわたる入力パターンの制御が不可能である。

ジェネレーターは、指定したセルに対して tick ごとに特定の bool パターンで値を注入する仕組み。

## 設計・方針

### ジェネレーターの位置づけ

ジェネレーターは **テスト基盤の概念** として check.json に定義する。回路定義 (circuit.json) には影響しない。

将来的にシミュレーション本体にジェネレーター機能を組み込む場合は、別タスクとして扱う。

### 制約

1. **ジェネレーターの出力先セルは、Wire の dst であってはならない**
   - incoming wire を持つセルにジェネレーターを接続すると、tick 内で値の上書きが発生し、シミュレーション結果が不定になる
   - テストランナーで `circuit.incoming_indices(target).is_empty()` を検証する
2. **同一セルに対して複数のジェネレーターを定義できない**
   - check.json の `generators` フィールドは `BTreeMap<String, Vec<bool>>` であり、キーの一意性により自然に保証される
3. **パターン長が ticks より短い場合、最後の値を保持する**
   - 例: `ticks: 5`, pattern `[true, false]` → tick 0: true, tick 1–4: false

### check.json スキーマ拡張

#### 現行スキーマ

```json
{
  "ticks": 1,
  "cases": [
    {
      "name": "case_name",
      "initial": { "0,0": true },
      "expected": { "1,0": false }
    }
  ]
}
```

#### 拡張後スキーマ

```json
{
  "ticks": 5,
  "cases": [
    {
      "name": "case_name",
      "ticks": 3,
      "initial": { "2,0": false },
      "generators": {
        "0,0": [true, false, true],
        "0,1": [false, true, false]
      },
      "expected": { "2,0": true }
    }
  ]
}
```

追加フィールド:

| フィールド | 位置 | 型 | 必須 | 説明 |
|---|---|---|---|---|
| `ticks` | case 内 | `usize` | No | ファイルレベル `ticks` のオーバーライド。省略時はファイルレベル値を使用 |
| `generators` | case 内 | `BTreeMap<String, Vec<bool>>` | No | セル座標 → tick ごとの値パターン |

- `generators` と `initial` は同一セルに対して共存可（`generators[0]` が `initial` を上書きする）
- `generators` のキーは `"x,y"` 形式（`initial` / `expected` と同一形式）

### テストランナーの実行フロー

```
1. circuit.json → Circuit を構築
2. check.json → テストケースを取得
3. Simulator::new(circuit)
4. initial の値を state_mut().set() で設定
5. for tick_i in 0..ticks:
     a. generators の各エントリについて:
        - pattern[min(tick_i, pattern.len() - 1)] の値を state_mut().set() で注入
     b. sim.tick()
6. expected の各値を検証
```

注意: ステップ 5a で `state_mut().set()` は `prev_state` と `curr_state` の両方を更新する。ジェネレーター対象セルは incoming wire を持たないため、tick 処理中に `curr_state[cell] = prev_state[cell]` が適用され、注入した値がそのまま保持される。

### バリデーション

テストランナーのセットアップ時に以下を検証する:

```rust
// generators の各ターゲットが incoming wire を持たないことを確認
for pos_str in test_case.generators.keys() {
    let pos = parse_pos(pos_str);
    assert!(
        circuit.incoming_indices(pos).is_empty(),
        "Generator target {} must not have incoming wires",
        pos_str
    );
}

// generators のパターンが空でないことを確認
for (pos_str, pattern) in &test_case.generators {
    assert!(
        !pattern.is_empty(),
        "Generator pattern for {} must not be empty",
        pos_str
    );
}
```

### Rust 側の型定義変更

`tests/test_helpers.rs`:

```rust
#[derive(serde::Deserialize)]
struct CheckFile {
    ticks: usize,
    cases: Vec<TestCase>,
}

#[derive(serde::Deserialize)]
struct TestCase {
    name: String,
    #[serde(default)]
    ticks: Option<usize>,  // 追加: per-case ticks オーバーライド
    #[serde(default)]
    initial: BTreeMap<String, bool>,
    #[serde(default)]
    generators: BTreeMap<String, Vec<bool>>,  // 追加
    #[serde(default)]
    expected: BTreeMap<String, bool>,
}
```

### build.rs 側の変更

build.rs では check.json から case 名を抽出するだけなので、`generators` フィールドの追加による変更は不要。ただし per-case `ticks` のフィールドが増えても `CaseEntry` は `name` のみを参照するため影響なし。

## validation テスト型の追加

失敗テスト用に新しいテスト型 `validation` を追加する。

### マニフェスト定義

```yaml
- name: self_loop
  type: validation
  path: validation/self_loop
  comment: "Self-loop wire is rejected"
```

### ディレクトリ構造

```
resources/tests/validation/self_loop/
  circuit.json     # 不正な回路定義
  expected.json    # 期待するエラーメッセージ
```

### expected.json スキーマ

```json
{
  "error_contains": "self-loop wire is not allowed"
}
```

### build.rs の拡張

```rust
fn write_validation_test(f: &mut fs::File, test: &TestCase) {
    let comment_line = /* 既存と同様 */;
    writeln!(
        f,
        r#"{}#[test]
fn test_{}_() {{
    test_validation_case("{}")
}}
"#,
        comment_line, test.name, test.path
    ).unwrap();
}
```

validation テストは 1 ディレクトリ = 1 テスト関数（case 分割なし）。

### test_helpers.rs: test_validation_case

```rust
pub fn test_validation_case(test_dir: &str) {
    let circuit_path = format!("resources/tests/{}/circuit.json", test_dir);
    let circuit_content = std::fs::read_to_string(&circuit_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", circuit_path));

    let expected_path = format!("resources/tests/{}/expected.json", test_dir);
    let expected_content = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", expected_path));

    #[derive(serde::Deserialize)]
    struct ExpectedError {
        error_contains: String,
    }

    let expected: ExpectedError = serde_json::from_str(&expected_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}", expected_path));

    let result = lgcell2_core::io::json::parse_circuit_json(&circuit_content);
    let err = result.expect_err(&format!(
        "Circuit in {} should be rejected, but was accepted",
        test_dir
    ));
    assert!(
        err.contains(&expected.error_contains),
        "Error message '{}' does not contain expected substring '{}'",
        err,
        expected.error_contains
    );
}
```
