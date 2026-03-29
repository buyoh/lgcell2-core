# Simulator OutputFormat 最適化: API 整理と Tick リプレイ

タスクの概要: `OutputFormat` 導入後も `cell_values()` / `get_cell()` など全セル前提の API が残存しており、内部で全セル値を保持し続ける必要がある状態を解消する。出力のキャッシュと Tick リプレイ機能を追加し、将来の ViewPort 最適化の基盤を作る。

作成日: 2026-03-29
ステータス: 完了

## 進捗

- 2026-03-29: phase1 を実装。`Simulator::cell_values()` と `Simulator::get_cell()` を削除し、呼び出し元を `last_output()` ベースに移行した
- 2026-03-29: phase1 を成立させる前提として `last_output` キャッシュと `replay_tick()` を先行導入した
- 2026-03-29: `TickOutput.tick` と JSON 出力の tick 番号を completed tick の 0-based インデックスに統一した
- 2026-03-29: `run_with_snapshots()` を出力キャッシュ clone ベースへ変更し、タスク全体を完了した

## 背景・動機

`OutputFormat::ViewPort` はシミュレーション結果の一部のみ収集する仕組みとして追加されたが、以下の問題がある:

1. **全セル前提の公開 API**: `cell_values()` は全セルの `HashMap<Pos, bool>` を返し、`get_cell(pos)` は任意のセルへのアクセスを提供する。これらが存在する限り、内部で全セル値を保持し続ける必要がある
2. **`build_output()` の都度計算**: `run_with_snapshots()` で呼ばれるたびに `HashMap` を構築するオーバーヘッドがある
3. **OutputFormat 切替時の確認手段がない**: `set_output_format()` で切り替えても、次の tick まで新しい形式の出力を確認できない

`OutputFormat` の本来の目的は、シミュレーション時も含めてすべてのセル値を保持する必要をなくすことである。まず外部 API から全セル前提を排除し、出力キャッシュとリプレイ機能で将来の最適化に備える。

## 設計・方針

### 1. 公開 API の変更

**削除するメソッド:**

| メソッド | 理由 | 現在の呼び出し元 |
|---------|------|---------------|
| `cell_values() -> HashMap<Pos, bool>` | 全セルを返す | view.rs, wasm_api, engine_tests |
| `get_cell(pos) -> Option<bool>` | 全セル保持を前提 | engine_tests, json_tests, test_helpers |

**追加するメソッド:**

| メソッド | 説明 |
|---------|------|
| `last_output() -> &TickOutput` | キャッシュされた最新の出力への参照 |
| `replay_tick()` | 現在の状態から出力を再構築する。`set_output_format()` 後に使用 |

**変更しないメソッド:**
- `set_cell()`: 入力注入用。セルの存在チェックはインデックスマップで行い、出力形式とは独立
- `current_tick()`, `circuit()`, `step()`, `tick()`, `run()` 等: 既存の制御・情報取得 API

### 2. TickOutput キャッシュ

Simulator に `last_output: TickOutput` フィールドを追加する。

- **初期化**: コンストラクタで初期状態（全セル `false`）の TickOutput を生成
- **更新タイミング**: `complete_tick()` 内で `build_output()` を呼びキャッシュ更新
- **`run_with_snapshots()`**: 毎 tick ごとにキャッシュから clone して Vec に追加（現在と同等の動作）

```rust
fn complete_tick(&mut self) {
    // ... 既存の wire_state 更新 ...
    self.last_output = self.build_output();
    self.cell_index = 0;
    self.tick += 1;
}
```

注意: `build_output()` 内で `self.tick` を使用しているため、`self.tick += 1` より前に呼び出す。現状 `build_output()` は `complete_tick()` の外で `tick += 1` 後に呼ばれているため、tick 番号の意味が変わる。`last_output.tick` は完了した tick のインデックス（0-based）を表すように修正する。

### 3. Tick リプレイ

```rust
/// 現在の状態から出力を再構築する。
/// `set_output_format()` で出力形式を変更した後に呼ぶことで、
/// tick を進めずに新しい形式の出力を確認できる。
pub fn replay_tick(&mut self) {
    self.last_output = self.build_output();
}
```

`cell_values: Vec<bool>` は tick 完了後も保持されているため、出力の再構築は `build_output()` の再呼び出しで実現できる。将来 `cell_values` を tick 完了後に破棄する最適化を行う場合は、`WireSimState` のスナップショットを保存して tick の再実行が必要になるが、それは別タスクとする。

### 4. 呼び出し元の更新

**`src/bin/lgcell2/view.rs`:**

```rust
// Before
let state = simulator.cell_values();
let frame = renderer.render_frame(&state, simulator.current_tick(), paused, cols, rows);

// After
let output = simulator.last_output();
let frame = renderer.render_frame(&output.cells, output.tick, paused, cols, rows);
```

初回レンダリング（tick 実行前）は `last_output()` が初期状態の TickOutput を返すため、特別な処理は不要。

**`src/wasm_api/simulator.rs`:**

```rust
// build_cell_states(): cell_values() → last_output()
fn build_cell_states(&self) -> Vec<WasmCellState> {
    self.simulator
        .last_output()
        .cells
        .iter()
        .map(|(pos, &value)| WasmCellState { x: pos.x, y: pos.y, value })
        .collect()
}

// get_cell(): simulator.get_cell() → last_output().cells.get()
pub fn get_cell(&self, x: i32, y: i32) -> Option<bool> {
    let pos = Pos::new(x, y);
    self.simulator.last_output().cells.get(&pos).copied()
}
```

**`src/io/json.rs`:**
- `simulate_to_output_json()`: `run_with_snapshots()` の戻り値 `Vec<TickOutput>` を使用しており、変更不要

**テスト (`engine_tests.rs`, `json_tests.rs`, `test_helpers.rs`, `renderer_tests.rs`):**
- `get_cell(pos)` → `last_output().cells.get(&pos).copied()`
- `cell_values()` → `last_output().cells.clone()` または `last_output().cells` への参照
- `renderer_tests.rs` の `make_state()`: `cell_values()` → `set_cell()` 後に `replay_tick()` で出力を構築し、`last_output().cells` を使用

### 5. TickOutput.tick の意味の整理

現状 `build_output()` で設定される `tick` は `self.tick` であり、`complete_tick()` で `self.tick += 1` された後の値が入る（`run_with_snapshots` 内で `tick()` → `build_output()` の順で呼ばれるため）。

キャッシュ化に伴い `complete_tick()` 内で `self.tick += 1` **前**に `build_output()` を呼ぶため、`TickOutput.tick` は完了した tick の 0-based インデックスになる。

これにより `verify_testers()` の `observed_tick = self.tick - 1` と整合性が取れる。

**影響を受けるテスト:**
```rust
// Before: tick 1 = "1回目の tick 完了後" (1-based)
assert_eq!(snapshots[0].tick, 1);

// After: tick 0 = "tick #0 完了後" (0-based)
assert_eq!(snapshots[0].tick, 0);
```

`SimulationOutputJson` の `tick` フィールドも同様に変更される。JSON 出力フォーマットの変更として `docs/spec/circuit-json.md` の更新が必要。

## ステップ

### フェーズ 1: API の削除

1. [x] **`cell_values()` 削除**: view.rs、wasm_api、テストの呼び出し元を更新
2. [x] **`get_cell()` 削除**: テスト、wasm_api の呼び出し元を更新

### フェーズ 2: 新しい API の追加（再構築など）

3. [x] **Simulator に `last_output` フィールド追加**: コンストラクタで初期化、`complete_tick()` でキャッシュ更新
4. [x] **`last_output()` アクセサ追加**: `&TickOutput` を返す
5. [x] **`replay_tick()` 追加**: `build_output()` 再呼び出しによる出力再構築
6. [x] **`TickOutput.tick` の意味を整理**: 0-based 化、テスト・JSON 出力の修正
7. [x] **呼び出し元を新 API に移行**: view.rs、wasm_api が `last_output()` を使用するよう更新

### フェーズ 3: 高速化対応

8. [x] **`run_with_snapshots()` 修正**: キャッシュから clone する方式に変更し、都度の `build_output()` 呼び出しを排除