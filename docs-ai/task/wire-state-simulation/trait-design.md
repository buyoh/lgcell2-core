# Simulator → WireSimulator 移行設計

旧 `Simulator` / `SimState` を削除し、`WireSimulator` / `WireSimState` で完全に置き換える。
共通 trait による共存は行わず、利用側を直接 `WireSimulator` に移行する。

## 削除対象

| 型・モジュール | 理由 |
|----------------|------|
| `simulation::SimState` | `WireSimState` に置き換え |
| `simulation::StateMut` | `WireSimulator::set_cell()` に置き換え |
| `simulation::Simulator` | `WireSimulator` に置き換え |

## API の置き換え対応表

| 旧 API | 新 API | 説明 |
|--------|--------|------|
| `Simulator::new(circuit)` | `WireSimulator::new(circuit)` | 生成 |
| `simulator.state().get(pos)` | `simulator.get_cell(pos)` | セル値取得 |
| `simulator.state_mut().set(pos, v)` | `simulator.set_cell(pos, v)` | セル値設定 |
| `simulator.state().values()` | `simulator.cell_values()` | 全セル値一覧 |
| `simulator.tick()` → `&SimState` | `simulator.tick()` → `()` | tick 実行 |
| `simulator.run(n)` → `&SimState` | `simulator.run(n)` → `()` | 複数 tick 実行 |
| `simulator.run_with_snapshots(n)` | 同名メソッドを `WireSimulator` に追加 | スナップショット取得 |

### `tick()` / `run()` の戻り値変更

旧実装は `prev_state`（`HashMap`）を直接借用して返していた。`WireSimulator` は `SimState` を持たないため `()` を返す。利用側は `tick()` 後に `get_cell()` / `cell_values()` で状態を取得する。

## 利用側の移行

### `wasm_api/simulator.rs` — `WasmSimulator`

旧実装では `Simulator` を直接フィールドとして保持していた。`WireSimulator` に差し替える:

```rust
pub struct WasmSimulator {
    simulator: WireSimulator,  // Simulator → WireSimulator
}
```

### `io/json.rs` — `simulate_to_output_json()`

`Simulator::run_with_snapshots()` を使用している箇所を `WireSimulator::run_with_snapshots()` に切り替える。戻り値の型が `TickSnapshot` のままであれば変更不要。

### `bin/lgcell2/main.rs` — CLI

`Simulator` の生成・実行箇所を `WireSimulator` に置き換える。

## trait 不要の判断

当初、旧実装と新実装の共存のために `SimulatorEngine` trait を導入する案があったが、以下の理由で不要と判断した:

- 旧実装は完全に削除するため、複数実装が同時に存在しない
- `WasmSimulator` は `wasm_bindgen` の制約上、ジェネリクスで export できないが、具体型 `WireSimulator` を直接保持すれば問題ない
- trait 抽象化は将来の拡張時に追加できる（YAGNI）
