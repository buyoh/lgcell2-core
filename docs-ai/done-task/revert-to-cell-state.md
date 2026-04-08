# Cell ベースシミュレーションへの回帰

`WireSimState`（遅延ワイヤ・入力なしセルのスロット管理）を廃止し、全セルの前 tick 値を `Vec<bool>` で保持するシンプルな Cell ベースモデルに再実装する。

作成日: 2026-04-08
ステータス: 完了

## 背景・動機

コミット 7ff39a9, 588d64d, 9e022a3 で旧 `SimState`（`HashMap<Pos, bool>` × 2、毎 tick clone）から `WireSimState`（遅延ワイヤ・入力なしセルのみスロット管理）+ `cell_values: Vec<bool>` に移行した。しかし以下の理由でメリットが少ない。

1. **責務の不一致**: Feedback 辺ではなく、Feedback 辺の out 側端点を持つ Cell が状態を持つべき
2. **頂点数 ≤ 辺数**: 一般に頂点（Cell）数より辺（Wire）数の方が多く、Wire ベースで状態を持つ方がメモリ的に不利になりうる
3. **改善幅が小さい**: 回路状態として既に `cell_values: Vec<bool>` を保持しているため、`WireSimState` の追加効果は最大でも 2 倍程度
4. **開発途上**: 実装はシンプルな方が保守性に優れる

### 注意事項

これは git revert ではなく再実装である。旧コミットのコードは参考にするが、移行後に追加された機能（`last_output` キャッシュ、`OutputFormat`、`is_updating()`、`replay_tick()` など）はすべて維持する。旧 `SimState`（HashMap ベース）に戻すのでもなく、現在の `Vec<bool>` アーキテクチャを土台として `WireSimState` 部分のみを置き換える。

## 設計・方針

### 変更の概要

現在の `Simulator` から `WireSimState` を除去し、`prev_cell_values: Vec<bool>` に置き換える。

| 項目 | 現在 (Wire ベース) | 新 (Cell ベース) |
|------|-------------------|-----------------|
| 前 tick 値の保持 | `WireSimState`（遅延ワイヤ・入力なしセルのスロットのみ） | `prev_cell_values: Vec<bool>`（全セル） |
| 遅延ワイヤの参照 | `wire_state.get_delayed_wire(wire_index)` | `prev_cell_values[src_idx]` |
| 入力なしセルの保持 | `wire_state.get_stateless_cell(cell_idx)` | `prev_cell_values[cell_idx]` |
| tick 完了時の更新 | 遅延ワイヤ・入力なしセルのスロットを個別更新 | `copy_from_slice` で一括コピー |
| `set_cell()` 時 | `wire_state` のスロット + ワイヤスロットを個別更新 | `prev_cell_values[index]` を更新 |

`cell_values`, `cell_pos_to_index`, `tick`, `cell_index`, `last_output`, `output_format` は変更なし。公開 API も変更なし。

### データモデル

```rust
pub struct Simulator {
    circuit: Circuit,
    prev_cell_values: Vec<bool>,            // 前 tick の全セル値（新規）
    cell_values: Vec<bool>,                 // 現在の tick で計算中の全セル値（既存）
    cell_pos_to_index: HashMap<Pos, usize>, // Pos → インデックスの逆引き（既存）
    tick: u64,                              // （既存）
    cell_index: usize,                      // （既存）
    last_output: TickOutput,                // （既存）
    output_format: OutputFormat,            // （既存）
}
```

### 各メソッドの変更

#### コンストラクタ (`with_output_format`)

`WireSimState::from_circuit()` を削除し、`prev_cell_values: vec![false; cell_count]` で初期化。それ以外の初期化ロジックは変更なし。

#### `step()`

遅延ワイヤの参照と入力なしセルの処理を変更。OR 合成・短絡評価のロジックは変更なし。

```rust
// 入力なしセル: prev_cell_values から値を引き継ぐ（入力対象セルは除く）
if incoming.is_empty() {
    if !self.circuit.inputs().iter().any(|i| i.target() == cell) {
        self.cell_values[cell_idx] = self.prev_cell_values[cell_idx];
    }
    // 入力対象セルは apply_inputs() で設定済み
}

// 遅延ワイヤ: WireSimState のスロット参照 → prev_cell_values の直接参照
let src_value = if wire.dst < wire.src {
    self.prev_cell_values[src_idx]   // 遅延伝搬: 前 tick の src 値
} else {
    self.cell_values[src_idx]        // 即時伝搬: 現 tick の src 値（変更なし）
};
```

#### `complete_tick()`

`wire_state` への選択的更新を、一括コピーに置き換える。`last_output` 構築と `tick` インクリメントは変更なし。

```rust
fn complete_tick(&mut self) {
    self.prev_cell_values.copy_from_slice(&self.cell_values);
    self.last_output = self.build_output();
    self.cell_index = 0;
    self.tick += 1;
}
```

#### `set_cell()`

`wire_state` のスロット更新・ワイヤスロット走査を、`prev_cell_values` の 1 要素更新に置き換える。`replay_tick()` 呼び出しは維持。

```rust
pub fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
    let index = self.cell_pos_to_index.get(&pos).copied()
        .ok_or(SimulationError::UnknownCell(pos))?;
    self.cell_values[index] = value;
    self.prev_cell_values[index] = value;
    self.replay_tick();
    Ok(())
}
```

#### `apply_inputs()`

変更なし。`cell_values` への書き込みのみで `wire_state` への操作はもともとない。

## ステップ

### 1. `engine.rs` の `Simulator` を Cell ベースに再実装

変更対象のフィールド・メソッド:
- フィールド: `wire_state: WireSimState` → `prev_cell_values: Vec<bool>`
- `with_output_format()`: 初期化変更
- `step()`: 遅延参照・入力なしセル処理の変更
- `complete_tick()`: `copy_from_slice` に変更
- `set_cell()`: `prev_cell_values` 更新に変更

変更なしのメソッド: `apply_inputs()`, `build_output()`, `tick()`, `run()`, `run_with_snapshots()`, `verify_testers()`, `run_with_verification()`, `circuit()`, `last_output()`, `replay_tick()`, `current_tick()`, `current_cell()`, `set_output_format()`, `is_updating()`

### 2. `wire_state.rs` を削除

`WireSimState` と `state_tests.rs` を削除する。

`WireSimState` は `simulation` モジュール外部からは参照されておらず、`engine.rs` と `mod.rs` のみが依存している。

### 3. `mod.rs` の更新

`pub mod wire_state` と `pub use wire_state::WireSimState` を削除。

### 4. WIP ファイルの更新 (`engine_gold.rs`, `engine_simple.rs`)

モジュールツリーに含まれていない WIP ファイルだが、`WireSimState` を使用しているため同様に Cell ベースに更新する。

### 5. ドキュメント更新

`docs-ai/architecture/simulation-model.md` の `WireSimState` 関連の記述を Cell ベースモデルに更新する。
