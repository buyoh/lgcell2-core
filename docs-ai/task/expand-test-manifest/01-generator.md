# ジェネレーター端子の設計

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

順序回路（SR ラッチ、JK フリップフロップ、カウンター等）のテストでは、tick ごとに入力信号を変化させる必要がある。現在の `initial` フィールドは tick 0 の初期値しか設定できず、複数 tick にわたる入力パターンの制御が不可能である。

ジェネレーターは、指定したセルに対して tick ごとに特定の bool パターンで値を注入する仕組み。テスト専用ではなく、通常の回路定義 (circuit.json) に組み込まれる回路モデルの一部として設計する。

## 設計・方針

### ジェネレーターの位置づけ

ジェネレーターは **回路モデルの一部** として circuit.json および `Circuit` 構造体に定義する。シミュレータが tick ごとにジェネレーターの値を自動適用する。

テスト用途では、check.json の per-case でジェネレーターを指定することで、回路定義のジェネレーターをオーバーライドできる。

### 制約

1. **ジェネレーターの出力先セルは、Wire の dst であってはならない**
   - incoming wire を持つセルにジェネレーターを接続すると、tick 内で値の上書きが発生し、シミュレーション結果が不定になる
   - `Circuit::new()` で `circuit.incoming_indices(target).is_empty()` を検証する
2. **同一セルに対して複数のジェネレーターを定義できない**
   - `Circuit::new()` で target の一意性を検証する
3. **パターンは空であってはならない**
   - `Circuit::new()` で pattern の長さ ≥ 1 を検証する

### パターン表記

パターンは文字列で表記する。`'1'` = true, `'0'` = false。

```
"101"  → [true, false, true]
"0"    → [false]
"1111" → [true, true, true, true]
```

`'0'`/`'1'` 以外の文字が含まれる場合はパースエラーとする。

### 繰り返しモード (loop)

パターン長が tick 数を超えた場合の動作を `loop` フラグで制御する。

| `loop` | 動作 | 例: pattern `"10"`, tick 0–4 |
|---|---|---|
| `false` (デフォルト) | 最後の値を保持 | 1, 0, 0, 0, 0 |
| `true` | 先頭に戻って繰り返す | 1, 0, 1, 0, 1 |

```rust
impl Generator {
    pub fn value_at(&self, tick: u64) -> bool {
        let idx = tick as usize;
        if self.is_loop {
            self.pattern[idx % self.pattern.len()]
        } else {
            self.pattern[idx.min(self.pattern.len() - 1)]
        }
    }
}
```

## データモデル変更

### 新規: Generator

`src/circuit/` に `generator.rs` を追加。

```rust
/// tick ごとに指定パターンで値を注入するジェネレーター。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generator {
    target: Pos,
    pattern: Vec<bool>,
    is_loop: bool,
}

impl Generator {
    pub fn new(target: Pos, pattern: Vec<bool>, is_loop: bool) -> Self {
        Self { target, pattern, is_loop }
    }

    pub fn target(&self) -> Pos { self.target }
    pub fn pattern(&self) -> &[bool] { &self.pattern }
    pub fn is_loop(&self) -> bool { self.is_loop }

    /// 指定 tick における出力値を返す。
    pub fn value_at(&self, tick: u64) -> bool {
        let idx = tick as usize;
        if self.is_loop {
            self.pattern[idx % self.pattern.len()]
        } else {
            self.pattern[idx.min(self.pattern.len() - 1)]
        }
    }
}
```

### Circuit 構造体の拡張

```rust
pub struct Circuit {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    generators: Vec<Generator>,           // 追加
    incoming: HashMap<Pos, Vec<usize>>,
    sorted_cells: Vec<Pos>,
}
```

後方互換性のため、既存の `Circuit::new()` はジェネレーターなしで構築する。新規 `Circuit::with_generators()` を追加:

```rust
impl Circuit {
    /// ジェネレーターなしで回路を構築する（既存互換）。
    pub fn new(cells: BTreeSet<Pos>, wires: Vec<Wire>) -> Result<Self, String> {
        Self::with_generators(cells, wires, Vec::new())
    }

    /// ジェネレーター付きで回路を構築する。
    pub fn with_generators(
        cells: BTreeSet<Pos>,
        wires: Vec<Wire>,
        generators: Vec<Generator>,
    ) -> Result<Self, String> {
        // 既存の wire バリデーション (self-loop, endpoint existence, duplicate wire)
        // ... 

        // --- 追加バリデーション ---
        // generator target が incoming wire を持たないこと
        for gen in &generators {
            if incoming.get(&gen.target()).map_or(false, |v| !v.is_empty()) {
                return Err(format!(
                    "generator target ({},{}) must not have incoming wires",
                    gen.target().x, gen.target().y
                ));
            }
        }
        // generator target の一意性
        let mut gen_targets = std::collections::HashSet::new();
        for gen in &generators {
            if !gen_targets.insert(gen.target()) {
                return Err(format!(
                    "duplicate generator target is not allowed: ({},{})",
                    gen.target().x, gen.target().y
                ));
            }
        }
        // pattern が空でないこと
        for gen in &generators {
            if gen.pattern().is_empty() {
                return Err(format!(
                    "generator pattern must not be empty: ({},{})",
                    gen.target().x, gen.target().y
                ));
            }
        }

        Ok(Self { cells, wires, generators, incoming, sorted_cells })
    }

    pub fn generators(&self) -> &[Generator] { &self.generators }
}
```

ジェネレーターの target セルは cells に自動追加する（wire の endpoint と同様）。

### Simulator の変更

tick ごとにジェネレーター値を適用する。適用タイミングは各 tick のセル処理開始前。

```rust
impl Simulator {
    fn apply_generators(&mut self) {
        for gen in self.circuit.generators() {
            let value = gen.value_at(self.tick);
            self.prev_state.set(gen.target(), value).unwrap();
            self.curr_state.set(gen.target(), value).unwrap();
        }
    }
}
```

`step()` の冒頭で `cell_index == 0` のとき `apply_generators()` を呼び出す:

```rust
fn step(&mut self) -> StepResult {
    if self.cell_index == 0 {
        self.apply_generators();
    }
    // ... 既存のセル処理
}
```

これにより `tick()` と `step()` のどちらを使っても一貫した動作となる。

## circuit.json スキーマ拡張

### 現行スキーマ

```json
{
  "wires": [...]
}
```

### 拡張後スキーマ

```json
{
  "wires": [...],
  "generators": [
    { "target": [0, 0], "pattern": "101" },
    { "target": [0, 1], "pattern": "010", "loop": true }
  ]
}
```

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `generators` | `GeneratorJson[]` | No | ジェネレーター定義の配列。省略時は空 |

GeneratorJson:

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `target` | `[i32, i32]` | Yes | 出力先セルの座標 |
| `pattern` | `string` | Yes | `'0'`/`'1'` の文字列パターン |
| `loop` | `bool` | No | `true` で先頭に戻って繰り返す。デフォルト `false`（最後の値を保持） |

### JSON → 内部モデル変換

`src/io/json.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
    #[serde(default)]
    pub generators: Vec<GeneratorJson>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorJson {
    pub target: [i32; 2],
    pub pattern: String,
    #[serde(default, rename = "loop")]
    pub is_loop: bool,
}
```

パターン文字列のパース:

```rust
fn parse_pattern(s: &str) -> Result<Vec<bool>, String> {
    s.chars()
        .map(|c| match c {
            '1' => Ok(true),
            '0' => Ok(false),
            _ => Err(format!("invalid pattern character: '{}' (expected '0' or '1')", c)),
        })
        .collect()
}
```

## check.json スキーマ拡張（テスト用）

テストでは check.json の per-case でジェネレーターを指定できる。circuit.json のジェネレーターと per-case ジェネレーターは target 単位でマージし、per-case が優先する。

### 拡張後スキーマ

```json
{
  "ticks": 5,
  "cases": [
    {
      "name": "case_name",
      "ticks": 3,
      "initial": { "2,0": false },
      "generators": [
        { "target": [0, 0], "pattern": "101" },
        { "target": [0, 1], "pattern": "010", "loop": true }
      ],
      "expected": { "2,0": true }
    }
  ]
}
```

追加フィールド:

| フィールド | 位置 | 型 | 必須 | 説明 |
|---|---|---|---|---|
| `ticks` | case 内 | `usize` | No | ファイルレベル `ticks` のオーバーライド |
| `generators` | case 内 | `GeneratorJson[]` | No | per-case ジェネレーター。circuit.json を target 単位でオーバーライド |

### テストランナーの実行フロー

```
1. circuit.json → wires + circuit_generators を取得
2. check.json → テストケースを取得
3. for each case:
   a. circuit_generators と case.generators を target 単位でマージ（case 優先）
   b. Circuit::with_generators(cells, wires, merged_generators)
   c. Simulator::new(circuit)
   d. initial の値を state_mut().set() で設定
   e. sim.run(ticks) で実行（ジェネレーターは Simulator が自動適用）
   f. expected の各値を検証
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
    ticks: Option<usize>,
    #[serde(default)]
    initial: BTreeMap<String, bool>,
    #[serde(default)]
    generators: Vec<GeneratorJson>,  // circuit.json と同一形式
    #[serde(default)]
    expected: BTreeMap<String, bool>,
}

#[derive(serde::Deserialize)]
struct GeneratorJson {
    target: [i32; 2],
    pattern: String,
    #[serde(default, rename = "loop")]
    is_loop: bool,
}
```

### build.rs 側の変更

build.rs では check.json から case 名を抽出するだけなので変更不要。

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
