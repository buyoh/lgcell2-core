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

### apply_inputs() の実装

tick 開始時に Generator 等の入力を `cell_values` に適用する。現行の `Simulator::apply_inputs()` は `prev_state` と `curr_state` の両方を更新しているが、`WireSimulator` では `cell_values` のみ更新する。

```rust
fn apply_inputs(&mut self) {
    for input in self.circuit.inputs() {
        let value = input.value_at(self.tick);
        let target = input.target();
        if let Some(&idx) = self.cell_pos_to_index.get(&target) {
            self.cell_values[idx] = value;
        }
    }
}
```

Generator 対象セルは `incoming_indices` が空であり、かつ Input として登録されているため、`from_circuit()` で「入力なしセル」としての遅延スロットは割り当てられない（`WireSimState::from_circuit()` で `has_input` チェックにより除外される）。

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
        // Generator 対象セルはスロットを持たないため、apply_inputs() で設定済みの値を保持
        if let Some(value) = self.wire_state.get_stateless_cell(cell_idx) {
            self.cell_values[cell_idx] = value;
        }
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

### cell_values()

```rust
fn cell_values(&self) -> Vec<(Pos, bool)> {
    self.circuit.sorted_cells().iter().enumerate()
        .map(|(idx, &pos)| (pos, self.cell_values[idx]))
        .collect()
}
```

### circuit() / current_tick() / current_cell()

現行 `Simulator` と同じアクセサをそのまま提供する。

```rust
fn circuit(&self) -> &Circuit {
    &self.circuit
}

fn current_tick(&self) -> u64 {
    self.tick
}

fn current_cell(&self) -> Option<Pos> {
    self.circuit.sorted_cells().get(self.cell_index).copied()
}
```

## テスター検証

現行の `Simulator` が提供する `verify_testers()` と `run_with_verification()` を `WireSimulator` でも提供する。ロジックは現行と同一だが、セル値の取得元が `prev_state.get()` から `cell_values[idx]` に変わる。

```rust
fn verify_testers(&self) -> Vec<TesterResult> {
    if self.tick == 0 {
        return Vec::new();
    }

    let observed_tick = self.tick - 1;
    let mut mismatches = Vec::new();
    for output in self.circuit.outputs() {
        match output {
            Output::Tester(tester) => {
                if let Some(expected) = tester.expected_at(observed_tick) {
                    let idx = self.cell_pos_to_index[&tester.target()];
                    let actual = self.cell_values[idx];
                    if actual != expected {
                        mismatches.push(TesterResult {
                            target: tester.target(),
                            tick: observed_tick,
                            expected,
                            actual,
                        });
                    }
                }
            }
        }
    }
    mismatches
}

fn run_with_verification(&mut self, ticks: u64) -> Vec<TesterResult> {
    let mut mismatches = Vec::new();
    for _ in 0..ticks {
        self.tick();
        mismatches.extend(self.verify_testers());
    }
    mismatches
}
```

## 出力形式

tick 完了時のセル状態の収集方法として AllCell / ViewPort の 2 形式を提供する。詳細は [output-format.md](output-format.md) を参照。

## テスト方針

`WireSimulator` はテストマニフェスト（`resources/tests/test-manifest.yaml`）を使い、エンジンテストで検証する。旧 `Simulator` との出力比較（クロステスト）は行わない。

テスト観点:
- tick 処理ごとのセル値がテストマニフェストの期待値と一致すること
- 遅延ワイヤ（後方ワイヤ）を含む回路での伝搬が正しいこと
- `set_cell()` / `get_cell()` が tick 前後で一貫した値を返すこと
