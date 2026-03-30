# コードレビュー 2026-03-26

タスクの概要: ソースコード、テストコード、設定ファイルを対象としたコードレビュー指摘事項の修正。

作成日: 2026-03-26
更新日: 2026-03-31
ステータス: 完了

## 背景・動機

`.github/copilot-instructions.md` の注意事項への違反と、一般的な設計原則に反する点を調査した結果、4 件の問題が検出された。
うち 2 件（項目 2・3）は別タスクで修正済み。残り 2 件（項目 1・4）を修正する。

## 検出された問題と対応方針

### 1. 回路構築ロジックの重複 → CircuitBuilder を導入

- **重要度**: medium
- **違反している原則**: DRY 原則（Don't Repeat Yourself）

**現状**:
以下の 3 箇所で「`BTreeSet<Pos>` を作成 → ワイヤ列を走査して `Wire` を構築しつつ `cells` に `src`/`dst` を挿入 → `Circuit::with_components()` を呼ぶ」というパターンが重複している。

1. `src/io/json.rs` — `TryFrom<CircuitJson> for Circuit` (L88–L155)
2. `src/wasm_api/simulator.rs` — `build_circuit_from_input()` (L134–L162)
3. `tests/test_helpers.rs` — `build_circuit_with_case_inputs()` (L140–L259)

**方針**: 案 A — `CircuitBuilder` を `circuit` モジュールに導入

```rust
/// ワイヤの追加時にセルを自動推論するビルダー。
pub struct CircuitBuilder {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
}

impl CircuitBuilder {
    pub fn new() -> Self { /* ... */ }
    /// ワイヤを追加し、src/dst をセルとして自動登録する。
    pub fn add_wire(&mut self, src: Pos, dst: Pos, kind: WireKind) -> &mut Self { /* ... */ }
    pub fn add_input(&mut self, input: Input) -> &mut Self { /* ... */ }
    pub fn add_output(&mut self, output: Output) -> &mut Self { /* ... */ }
    pub fn build(self) -> Result<Circuit, CircuitError> {
        Circuit::with_components(self.cells, self.wires, self.inputs, self.outputs)
    }
}
```

- 呼び出し元は `BTreeSet<Pos>` の管理やワイヤ端点からのセル挿入が不要になる
- `with_components()` / `with_generators()` / `new()` はそのまま残す（直接構築のユースケースも維持）
- 影響範囲: `circuit` モジュールに `CircuitBuilder` 追加、上記 3 箇所を書き換え

---

### 2. ~~`engine.rs` のプロダクションコードにおける `expect()` の使用~~ → ほぼ解消済み

- **重要度**: ~~medium~~ → low
- **ステータス**: ほぼ修正済み

**現状**:
レビュー時点で 8 箇所あった `expect()` は、別タスク（ワイヤ状態モデルのリファクタリング）で大半が削除された。
残り 1 箇所のみ:

- `src/simulation/engine.rs` L281: `.expect("delayed wire must have slot")`

これは `Circuit` 構築時の不変条件（遅延ワイヤには必ずスロットが存在する）に依存しており、入力バリデーションの問題ではなく実装上の不変条件であるため、`debug_assert!` + `unwrap_or` に置き換える。

```rust
// 変更前
self.wire_state
    .get_delayed_wire(wire_index)
    .expect("delayed wire must have slot")

// 変更後
let delayed_value = self.wire_state.get_delayed_wire(wire_index);
debug_assert!(delayed_value.is_some(), "delayed wire must have slot");
delayed_value.unwrap_or(false)
```

---

### 3. ~~`Circuit::with_generators()` の責務過多~~ → 解消済み

- **重要度**: ~~low~~ → なし
- **ステータス**: 修正済み

`with_generators()` は 5 行の薄いラッパーにリファクタリングされ、すべての責務は `with_components()` に委譲された。追加対応不要。

---

### 4. `build.rs` のエラーハンドリング不統一 → `.expect()` に統一

- **重要度**: low
- **違反している原則**: コードの一貫性

**現状**:
`build.rs` 内で `.unwrap()` と `.expect()` が混在している:

| 行 | 現在のパターン | 内容 | 変更後 |
|---|---|---|---|
| L38 | `.unwrap()` | `OUT_DIR` 環境変数の取得 | `.expect("OUT_DIR not set")` |
| L40 | `.unwrap()` | ファイル作成 | `.expect("Failed to create generated_tests.rs")` |
| L88 | `.unwrap()` | `writeln!` の結果 | `.expect("Failed to write test function")` |
| L108 | `.unwrap()` | `writeln!` の結果 | `.expect("Failed to write test function")` |

`.unwrap_or_else(|_| panic!())` は既にメッセージ付きなのでそのまま維持。

影響範囲: `build.rs` のみ

## ステップ

1. `CircuitBuilder` を `src/circuit/` に追加し、`mod.rs` で公開する
2. `src/io/json.rs` を `CircuitBuilder` を使うようにリファクタリング
3. `src/wasm_api/simulator.rs` を `CircuitBuilder` を使うようにリファクタリング
4. `tests/test_helpers.rs` を `CircuitBuilder` を使うようにリファクタリング
5. `src/simulation/engine.rs` L186 の `expect()` を `debug_assert!` + `unwrap_or` に変更
6. `build.rs` の `.unwrap()` を `.expect("説明")` に統一
7. テスト実行で全テスト通過を確認
