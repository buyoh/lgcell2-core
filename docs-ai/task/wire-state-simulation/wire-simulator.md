# WireSimulator — ワイヤ状態モデルのシミュレーションエンジン

## データ構造

```rust
#[derive(Debug, Clone)]
pub struct WireSimulator {
    circuit: Circuit,
    /// 遅延ワイヤベースの状態（tick 間持続）。
    wire_state: WireSimState,
    /// 現在 tick のセル計算値。sorted_cells と同じ順序・サイズ。
    cell_values: Vec<bool>,
    /// セル座標 → sorted_cells インデックス（構築時計算）。
    cell_pos_to_index: HashMap<Pos, usize>,
    /// 現在の tick 番号 (0-origin)。
    tick: u64,
    /// 現在の tick 内で次に処理すべきセルのインデックス。
    cell_index: usize,
}
```

### `cell_values` の役割

`cell_values` は sorted_cells と同サイズの `Vec<bool>` で、現在 tick の処理中にセルの計算結果を蓄積する。

- **tick 処理中**: セルが処理されるたびに `cell_values[cell_index]` を更新
- **tick 完了後**: 完了状態として参照可能（`get_cell()`, `cell_values()` 経由）
- **次 tick 開始時**: `cell_values` はリセットせず、tick 処理で上書きされる

### `cell_pos_to_index` の役割

`HashMap<Pos, usize>` で座標からインデックスへのルックアップを提供する。構築時に一度だけ計算。

## 構築

```rust
impl WireSimulator {
    pub fn new(circuit: Circuit) -> Self {
        let wire_state = WireSimState::from_circuit(&circuit);
        let cell_count = circuit.sorted_cells().len();
        let cell_values = vec![false; cell_count];
        let cell_pos_to_index: HashMap<Pos, usize> = circuit
            .sorted_cells()
            .iter()
            .enumerate()
            .map(|(i, &pos)| (pos, i))
            .collect();

        Self {
            circuit,
            wire_state,
            cell_values,
            cell_pos_to_index,
            tick: 0,
            cell_index: 0,
        }
    }
}
```

## tick 処理

### step() の疑似コード

```rust
fn step(&mut self) -> StepResult {
    if self.cell_index == 0 {
        self.apply_inputs();
    }

    let cell_idx = self.cell_index;
    let cell = self.circuit.sorted_cells()[cell_idx];
    let incoming = self.circuit.incoming_indices(cell);

    if incoming.is_empty() {
        // 入力なしセル: 遅延スロットから前 tick の値を復元
        let value = self.wire_state
            .get_stateless_cell(cell_idx)
            .unwrap_or(false);
        self.cell_values[cell_idx] = value;
    } else {
        let mut next_value = false;
        for &wire_index in incoming {
            let wire = &self.circuit.wires()[wire_index];
            let src_value = if wire.dst < wire.src {
                // 遅延ワイヤ: wire_state から前 tick のソース値を取得
                let raw = self.wire_state
                    .get_delayed_wire(wire_index)
                    .expect("delayed wire must have slot");
                raw
            } else {
                // 即時ワイヤ: cell_values からソース値を取得
                let src_idx = self.cell_pos_to_index[&wire.src];
                self.cell_values[src_idx]
            };

            next_value = next_value || wire.propagate(src_value);
            if next_value { break; }  // short-circuit OR
        }
        self.cell_values[cell_idx] = next_value;
    }

    self.cell_index += 1;
    if self.cell_index >= self.circuit.sorted_cells().len() {
        self.complete_tick();
        StepResult::TickComplete
    } else {
        StepResult::Continue
    }
}
```

### tick 完了処理

```rust
fn complete_tick(&mut self) {
    // 遅延ワイヤのソース値を更新
    for (wire_index, wire) in self.circuit.wires().iter().enumerate() {
        if wire.dst < wire.src {
            let src_idx = self.cell_pos_to_index[&wire.src];
            self.wire_state.update_wire(wire_index, self.cell_values[src_idx]);
        }
    }

    // 入力なしセルの値を更新
    for (cell_idx, &pos) in self.circuit.sorted_cells().iter().enumerate() {
        if self.circuit.incoming_indices(pos).is_empty()
            && !self.circuit.inputs().iter().any(|i| i.target() == pos)
        {
            self.wire_state.update_cell(cell_idx, self.cell_values[cell_idx]);
        }
    }

    self.cell_index = 0;
    self.tick += 1;
}
```

**注意**: `complete_tick()` では全セルのクローンを行わない。`wire_state` の遅延スロットのみ更新する。

## 遅延ワイヤの値の解釈

重要な設計判断として、`wire_state` に格納する値は **伝搬前のソースセル値**（`cell_values[src_idx]`）である。伝搬（極性の適用）は `step()` で `wire.propagate()` を呼ぶ際に行う。

理由:
- 同一ソースセルから複数の遅延ワイヤが出る場合、ソース値を 1 回だけ保存すれば済む
  - ただし現行設計では (src, dst) ペアの一意性制約があるため、同一ソースからの遅延ワイヤは dst ごとに 1 本
  - 将来的に最適化の余地を残すため、ソース値ベースで格納する

## state 互換 API

### get_cell()

```rust
fn get_cell(&self, pos: Pos) -> Option<bool> {
    self.cell_pos_to_index.get(&pos).map(|&idx| self.cell_values[idx])
}
```

### set_cell()

```rust
fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
    let idx = self.cell_pos_to_index.get(&pos)
        .ok_or(SimulationError::UnknownCell(pos))?;
    self.cell_values[*idx] = value;

    // 遅延スロットも更新（このセルが遅延ワイヤのソースまたは入力なしセルの場合）
    self.wire_state.update_cell(*idx, value);
    for (wire_index, wire) in self.circuit.wires().iter().enumerate() {
        if wire.src == pos && wire.dst < wire.src {
            self.wire_state.update_wire(wire_index, value);
        }
    }

    Ok(())
}
```

`set_cell()` は `cell_values` と `wire_state` の両方を同期的に更新する。これにより、tick 開始前のどのタイミングで呼ばれても一貫した状態が保たれる。

## 現行 `Simulator` との等価性

同一回路で `Simulator` と `WireSimulator` を実行した場合、全 tick の全セル値は完全に一致する。これをクロステストで検証する。

```rust
#[test]
fn wire_simulator_matches_cell_simulator() {
    let circuit = /* テスト回路 */;
    let mut old = Simulator::new(circuit.clone());
    let mut new = WireSimulator::new(circuit);

    for _ in 0..100 {
        old.tick();
        new.tick();
        for &pos in old.circuit().sorted_cells() {
            assert_eq!(
                old.state().get(pos),
                new.get_cell(pos),
                "mismatch at tick {} pos {:?}",
                old.current_tick(), pos
            );
        }
    }
}
```
