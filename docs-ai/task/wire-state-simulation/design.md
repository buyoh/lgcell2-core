# ワイヤ状態モデル — 詳細設計

## 概念モデルの比較

### 旧モデル：セル状態

```
状態 = HashMap<Pos, bool>  (全セル × 2バッファ)

tick 処理:
  for each cell in sorted_cells:
    wire が後退 (dst < src) → prev_state[src] を参照
    wire が前進 (src < dst) → curr_state[src] を参照（処理済み）
    curr_state[cell] = OR(全入力ワイヤの伝搬値)
  prev_state ← curr_state.clone()
```

セル数が N の場合、状態サイズ = 2N bits。

### 新モデル：ワイヤ状態

```
状態 = HashMap<WireIndex, bool>  (後退ワイヤのみ)

tick 処理:
  cell_buf = 一時バッファ (HashMap<Pos, bool>)
  for each cell in sorted_cells:
    wire が後退 (dst < src) → wire_state[wire_index] を参照
    wire が前進 (src < dst) → cell_buf[src] を参照（処理済み）
    cell_buf[cell] = OR(全入力ワイヤの伝搬値)
  // tick 末尾: 後退ワイヤの状態を更新
  for each backward wire:
    wire_state[wire_index] = wire.propagate(cell_buf[wire.src])
  // スナップショット用に cell_buf を保存
  last_snapshot ← cell_buf
```

後退ワイヤ数を B、全ワイヤ数を W、セル数を N とすると、永続状態サイズ = B bits (B ≤ W ≤ N²)。

### なぜワイヤ状態が正しいか

- **前進ワイヤ** (`src < dst`): src が先に処理されるため、dst を処理する時点で src の今 tick の値が確定している。状態なし。
- **後退ワイヤ** (`dst < src`): dst が先に処理されるため、src の今 tick の値はまだ不明。よって前 tick の「src から dst に伝搬した値」を記憶する必要がある。

後退ワイヤこそが回路の「記憶素子」であり、セル自体は状態を持たない。

---

## データ構造設計

### `WireState`

```rust
/// 後退ワイヤ（dst < src）が保持する遅延伝搬値。
/// ticker 間の境界で更新される。
pub struct WireState {
    /// ワイヤインデックス → 前 tick に伝搬した値
    /// 後退ワイヤのみをエントリとして持つ。
    values: HashMap<usize, bool>,
}

impl WireState {
    /// 回路から後退ワイヤを抽出して初期化する。初期値は全て false。
    pub fn from_circuit(circuit: &Circuit) -> Self;

    /// 後退ワイヤの値を取得する。
    pub fn get(&self, wire_index: usize) -> Option<bool>;

    /// 後退ワイヤの値を更新する。
    pub fn set(&mut self, wire_index: usize, value: bool) -> Result<(), SimulationError>;

    /// 後退ワイヤのインデックス一覧を返す。
    pub fn backward_wire_indices(&self) -> impl Iterator<Item=usize> + '_;
}
```

ファイル: `src/simulation/wire_state.rs`

### `WireSimulator`

```rust
/// ワイヤ状態モデルによるシミュレーションエンジン。
pub struct WireSimulator {
    circuit: Circuit,
    /// 後退ワイヤの遅延伝搬値（tick 間で永続）。
    wire_state: WireState,
    /// 現在 tick の計算バッファ（tick 内でのみ有効）。
    cell_buf: HashMap<Pos, bool>,
    /// 直前に完了した tick のセル値スナップショット（外部公開用）。
    last_state: SimState,
    /// 現在の tick 番号（0-origin）。
    tick: u64,
    /// 現在の tick 内で次に処理すべきセルのインデックス。
    cell_index: usize,
}
```

ファイル: `src/simulation/wire_engine.rs`

#### `step()` アルゴリズム

```
step():
  if cell_index == 0:
    apply_inputs()   // Input コンポーネントを cell_buf と wire_state に反映

  cell = sorted_cells[cell_index]
  incoming = circuit.incoming_indices(cell)

  if incoming.is_empty():
    // 入力なし → 前 tick の値を保持
    // last_state から取得（または wire_state で入力セルを管理）
    cell_buf[cell] = last_state.get(cell)  // false if first tick

  else:
    value = false
    for wire_index in incoming:
      wire = wires[wire_index]
      src_val = if wire.dst < wire.src:
        wire_state.get(wire_index)      // 後退ワイヤ → 遅延値
      else:
        cell_buf[wire.src]              // 前進ワイヤ → 今 tick の計算済み値
      value = value || wire.propagate(src_val)
      if value: break
    cell_buf[cell] = value

  cell_index += 1
  if cell_index >= sorted_cells.len():
    // tick 完了 → 後退ワイヤ状態を更新
    for wire_index in wire_state.backward_wire_indices():
      wire = wires[wire_index]
      new_val = wire.propagate(cell_buf[wire.src])
      wire_state.set(wire_index, new_val)
    last_state = SimState::from_cell_buf(&cell_buf, circuit)
    cell_buf.clear()   // 次 tick のためにクリア（またはリセット）
    cell_index = 0
    tick += 1
    TickComplete
  else:
    Continue
```

#### `apply_inputs()`

入力セルは Input コンポーネント (`Generator` 等) が値を決める。
`cell_buf[input.target()] = input.value_at(tick)` を設定する。
また、入力セルから出る後退ワイヤ状態も即座に更新する（tick 0 から正しく動作するため）。

※ 旧 `Simulator` の `apply_inputs()` と等価だが、更新先が `cell_buf` と `wire_state` に変わる。

---

## 共通トレイト設計

### `Simulate`

```rust
/// シミュレーションエンジンの共通インターフェース。
pub trait Simulate {
    /// 1 セル分だけ進める。
    fn step(&mut self) -> StepResult;

    /// 1 tick 完了まで進め、その tick のセル状態を返す。
    fn tick(&mut self) -> &SimState;

    /// 指定 tick 数だけ進め、最後の tick のセル状態を返す。
    fn run(&mut self, ticks: u64) -> &SimState;

    /// 指定 tick 数だけ進め、各 tick のスナップショットを返す。
    fn run_with_snapshots(&mut self, ticks: u64) -> Vec<TickSnapshot>;

    /// 直近で完了した tick のテスター検証を行い、不一致を返す。
    fn verify_testers(&self) -> Vec<TesterResult>;

    /// 指定 tick 数だけ進め、各 tick のテスター結果を収集する。
    fn run_with_verification(&mut self, ticks: u64) -> Vec<TesterResult>;

    /// 回路定義を取得する。
    fn circuit(&self) -> &Circuit;

    /// 直前に完了した tick のセル状態を返す。
    fn state(&self) -> &SimState;

    /// 入力セルの値を設定するためのヘルパーを返す。
    fn state_mut(&mut self) -> StateMut<'_>;

    /// 現在の tick 番号を返す。
    fn current_tick(&self) -> u64;

    /// 現在 tick 内で次に処理するセルを返す。
    fn current_cell(&self) -> Option<Pos>;
}
```

ファイル: `src/simulation/simulate.rs`

### 旧 `Simulator` への実装

旧 `Simulator` は現在の `tick()`・`run()` 等のメソッドをそのまま持っているため、
トレイトの実体化は概ねメソッドの再マッピングになる。
旧 `Simulator` の `state()` は `&self.prev_state` を返すため `SimState` 互換。

### `WireSimulator` への実装

`tick()` / `state()` は `&self.last_state` を返す。
`last_state` は tick 完了時に `cell_buf` から構築した `SimState` のスナップショット。

---

## ファイル構成（変更後）

```
src/simulation/
├── mod.rs              // 公開 API 更新
├── simulate.rs         // NEW: Simulate トレイト, StateMut
├── state.rs            // SimState（変更なし）
├── state_tests.rs
├── wire_state.rs       // NEW: WireState
├── wire_state_tests.rs // NEW
├── engine.rs           // 旧 Simulator（変更: Simulate トレイトを実装）
├── engine_tests.rs     // 変更: Simulate 経由のテスト追加
├── wire_engine.rs      // NEW: WireSimulator
└── wire_engine_tests.rs// NEW
```

---

## 移行・互換方針

- 旧 `Simulator` は **削除しない**。`Simulate` トレイトを実装し、明示的に旧実装として残す。
- 外部 API (`wasm_api/`, `io/`, `bin/`) は `Simulate` トレイト経由に統一することを目指す。
  - 当面は `Simulator` のデフォルト実装を維持し、段階的に移行する。
- `SimState` の構造は変更しない。`WireSimulator` は tick 完了時に `SimState` スナップショットを構築して外部に公開する。

---

## 考慮事項・トレードオフ

| 観点 | 旧 (Simulator) | 新 (WireSimulator) |
|---|---|---|
| 永続状態サイズ | 2 × N bits (全セル×2) | B bits (後退ワイヤ数) |
| 一時メモリ | 不要 | 1 × N bits (cell_buf) |
| 実装複雑度 | 低 | やや高い（tick 末尾での wire_state 更新が必要）|
| 概念的正確性 | 旧仕様 (セルが状態保持) | 現仕様 (ワイヤが状態保持) |
| 外部 API 互換性 | 現行のまま | SimState スナップショット方式で互換維持 |

---

## 関連ファイル

- `src/simulation/engine.rs` — 旧 `Simulator`
- `src/simulation/state.rs` — `SimState`
- `docs-ai/architecture/simulation-model.md` — シミュレーションモデルの解説
