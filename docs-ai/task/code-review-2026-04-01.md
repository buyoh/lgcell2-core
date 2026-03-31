# コードレビュー 2026-04-01

タスクの概要: ソースコード、テストコード、設定ファイルを対象としたコードレビュー。

作成日: 2026-04-01
ステータス: 未着手

---

## 1. `io/json.rs` と `parser/json.rs` の完全重複 — wasm ビルド不能

- **重要度**: high
- **違反しているルール**: DRY 原則、モジュール境界の整合性

### 問題点

`src/io/json.rs`（222 行）と `src/parser/json.rs`（222 行）は完全に同一のコードである。`diff` で差分ゼロ。

さらに `src/io/` ディレクトリは `lib.rs` でモジュール宣言されておらず、`wasm` feature 有効時に `wasm_api/legacy.rs` および `wasm_api/simulator.rs` が `crate::io::json` を参照するため、**wasm ビルドがコンパイルエラーになる**。

```
error[E0433]: failed to resolve: unresolved import
 --> src/wasm_api/legacy.rs:3:12
  |
3 | use crate::io::json::{output_json_to_string, parse_circuit_json, simulate_to_output_json};
  |            ^^ unresolved import
```

### 解決策

**案 A（推奨）: `src/io/` を削除し、wasm_api の参照先を `parser::json` に変更**

- `src/io/json.rs` を削除
- `src/wasm_api/legacy.rs` と `src/wasm_api/simulator.rs` の `use crate::io::json` を `use crate::parser::json` に変更

影響範囲: `src/io/json.rs`（削除）、`src/wasm_api/legacy.rs`、`src/wasm_api/simulator.rs`

**案 B: `lib.rs` に `mod io` を追加し、`parser` を `io` の再エクスポートにする**

- `lib.rs` に `pub(crate) mod io;` と `src/io/mod.rs` を追加
- `parser/json.rs` を削除し、`parser/mod.rs` で `pub use crate::io::json;` にする

影響範囲: `src/lib.rs`、`src/io/mod.rs`（新設）、`src/parser/mod.rs`、`src/parser/json.rs`（削除）

---

## 2. `WireSimState` が public API としてエクスポートされている

- **重要度**: medium
- **違反しているルール**: 情報隠蔽原則（カプセル化）

### 問題点

`src/simulation/mod.rs` で `WireSimState` が `pub use` されている。`WireSimState` は `SimulatorSimple` の内部実装詳細であり、外部コードから直接操作される想定ではない。

```rust
// src/simulation/mod.rs
pub use wire_state::WireSimState;
```

現状、`WireSimState` を外部から使用しているコードは存在しない（`SimulatorSimple` 内部でのみ使用）。

### 解決策

`pub use wire_state::WireSimState;` を削除し、`wire_state` モジュール自体を `pub(crate)` または非公開にする。

影響範囲: `src/simulation/mod.rs` のみ（外部呼び出し元なし）

---

## 3. `test_helpers.rs` のエラーマスキング

- **重要度**: medium
- **違反しているルール**: デバッグ容易性・エラーコンテキスト保持

### 問題点

`tests/test_helpers.rs` で `.unwrap_or_else(|_| panic!("..."))` パターンが使われており、`|_|` によって元のエラー情報が破棄される。テスト失敗時にファイルが見つからないのかパースエラーなのか、具体的な原因がわからない。

```rust
// L67-68
let circuit_content = std::fs::read_to_string(&circuit_path)
    .unwrap_or_else(|_| panic!("Failed to read {}", circuit_path));

let circuit_json: CircuitJson = serde_json::from_str(&circuit_content)
    .unwrap_or_else(|_| panic!("Failed to parse {}", circuit_path));
```

該当箇所: L67, L72, L75, L78, L82, L130, L135, L138

### 解決策

`|_|` を `|e|` に変更し、エラーメッセージにエラー内容を含める。

```rust
let circuit_content = std::fs::read_to_string(&circuit_path)
    .unwrap_or_else(|e| panic!("Failed to read {}: {}", circuit_path, e));
```

影響範囲: `tests/test_helpers.rs` のみ

---

## 4. `Generator` / `Tester` の空パターン構築によるパニック

- **重要度**: medium
- **違反しているルール**: 防御的プログラミング、システム境界でのバリデーション

### 問題点

`Generator::new()` および `Tester::new()` は空の `pattern` / `expected` を受け入れるが、`value_at()` / `expected_at()` は空パターンでパニックする。

```rust
// Generator::value_at() — len が 0 の場合、tick % 0 でパニック
let len = self.pattern.len() as u64;
if self.is_loop {
    self.pattern[(tick % len) as usize]  // panic: division by zero
}
```

`Circuit::with_components()` で空パターンは検証されるが、`Generator`/`Tester` を直接構築するコードでは保証がない。

### 解決策

**案 A（推奨）: コンストラクタに `assert!` を追加**

```rust
impl Generator {
    pub fn new(target: Pos, pattern: Vec<bool>, is_loop: bool) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        Self { target, pattern, is_loop }
    }
}
```

**案 B: コンストラクタを `Result` に変更**

影響範囲: 案 A は `src/circuit/input_com/generator.rs`、`src/circuit/output_com/tester.rs` のみ。案 B は呼び出し元すべてに影響。

---

## 5. `build_output` ロジックの重複（`engine_simple.rs`）

- **重要度**: low
- **違反しているルール**: DRY 原則

### 問題点

`SimulatorSimple` の初期化時（L64–L80）と `build_output()`（L138–L157）で、`OutputFormat` に応じたセル収集ロジックが重複している。初期化時は値がすべて `false` 固定、`build_output()` は `cell_values[index]` を参照する点のみが異なる。

### 解決策

初期化時に `cell_values` を先に作成してから `build_output()` を呼び出すようにリファクタリングする。

```rust
pub fn with_output_format(circuit: Circuit, output_format: OutputFormat) -> Self {
    let cell_count = circuit.sorted_cells().len();
    let cell_values = vec![false; cell_count];
    // ... cell_pos_to_index の構築 ...
    let mut sim = Self {
        wire_state: WireSimState::from_circuit(&circuit),
        circuit, cell_values, cell_pos_to_index,
        tick: 0, cell_index: 0,
        last_output: TickOutput { tick: 0, cells: HashMap::new() }, // 仮
        output_format,
    };
    sim.last_output = sim.build_output();
    sim
}
```

影響範囲: `src/simulation/engine_simple.rs` のみ

---

## 6. `build.rs` 内の構造体にドキュメントコメントがない

- **重要度**: low
- **違反しているルール**: copilot-instructions.md「構造体の概要は必ずドキュメントコメントとして追加する」

### 問題点

`build.rs` 内の `TestManifest` と `TestCase` にドキュメントコメントがない。同じ違反が `build.rs` 内の `CheckFile` と `CaseEntry`（L65–L73）にもある。

```rust
#[derive(Debug, Deserialize)]
struct TestManifest {   // ← ドキュメントコメントなし
    tests: Vec<TestCase>,
}
```

### 解決策

ドキュメントコメントを追加する。

影響範囲: `build.rs` のみ

---

## 7. `wasm_api/simulator.rs` の `current_tick()` で u64 → u32 キャストが無警告で切り捨て

- **重要度**: low
- **違反しているルール**: 型安全性

### 問題点

```rust
pub fn current_tick(&self) -> u32 {
    // u64 → u32: Web 用途では 2^32 tick を超えることは想定しない
    self.simulator.current_tick() as u32
}
```

コメントで想定はされているが、2^32 tick 超過時に無警告で切り捨てが発生する。

### 解決策

**案 A: `u64` をそのまま返す** — wasm-bindgen は `u64` を `BigInt` としてバインドするため互換性あり

**案 B: `u32::try_from()` に変更し、オーバーフロー時にエラーを返す**

影響範囲: `src/wasm_api/simulator.rs`、および JavaScript 側の呼び出しコード（`lgcell2-webui`）
