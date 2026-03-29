# Simulator → WireSimulator 移行設計

旧 `Simulator` / `SimState` を削除し、`WireSimulator` / `WireSimState` で完全に置き換える。
共通 trait による共存は行わず、利用側を直接 `WireSimulator` に移行する。

## 削除対象

| 型・モジュール | 理由 |
|----------------|------|
| `simulation::SimState` | `WireSimState` に置き換え |
| `simulation::StateMut` | `WireSimulator::set_cell()` に置き換え |
| `simulation::Simulator` | `WireSimulator` に置き換え |
| `simulation::TickSnapshot` | `TickOutput` に置き換え（output-format.md 参照） |

## API の置き換え対応表

| 旧 API | 新 API | 説明 |
|--------|--------|------|
| `Simulator::new(circuit)` | `WireSimulator::new(circuit)` | 生成 |
| `simulator.step()` → `StepResult` | 同名メソッド | 1 セル処理（中断ポイント） |
| `simulator.state().get(pos)` | `simulator.get_cell(pos)` | セル値取得 |
| `simulator.state_mut().set(pos, v)` | `simulator.set_cell(pos, v)` | セル値設定 |
| `simulator.state().values()` | `simulator.cell_values()` | 全セル値一覧 |
| `simulator.tick()` → `&SimState` | `simulator.tick()` → `()` | tick 実行 |
| `simulator.run(n)` → `&SimState` | `simulator.run(n)` → `()` | 複数 tick 実行 |
| `simulator.run_with_snapshots(n)` | 同名メソッドを `WireSimulator` に追加 | スナップショット取得 |
| `simulator.verify_testers()` | 同名メソッドを `WireSimulator` に追加 | テスター検証 |
| `simulator.run_with_verification(n)` | 同名メソッドを `WireSimulator` に追加 | 複数 tick + テスター検証 |
| `simulator.circuit()` | 同名メソッド | 回路定義取得 |
| `simulator.current_tick()` | 同名メソッド | tick 番号取得 |
| `simulator.current_cell()` | 同名メソッド | 処理中セル取得 |

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

主な変更点:
- `get_cell()`: `simulator.state().get(pos)` → `simulator.get_cell(pos)`
- `set_cell()`: `simulator.state_mut().set(pos, v)` → `simulator.set_cell(pos, v)`
- `get_state()` / `build_cell_states()`: `simulator.state()` → `simulator.cell_values()` 等に切り替え
- `run_steps()`: `simulator.step()` → 同名メソッド（`StepResult` は同じ）

### `io/json.rs` — `simulate_to_output_json()`

`Simulator::run_with_snapshots()` を使用している箇所を `WireSimulator::run_with_snapshots()` に切り替える。戻り値の型が `TickSnapshot` から `TickOutput` に変わるため、`TickStateJson` への変換ロジックを調整する（`cells` が `Vec<(Pos, bool)>` → `HashMap<Pos, bool>` に変更）。

### `bin/lgcell2/main.rs` — CLI

`Simulator` の生成・実行箇所を `WireSimulator` に置き換える。

### `bin/lgcell2/view.rs` — ビューモード

`Simulator::new()`, `simulator.tick()`, `simulator.state()`, `simulator.current_tick()` を使用している。`WireSimulator` に差し替える。

`render_once()` 内で `simulator.state()` → `ViewRenderer::render_frame()` に渡しているため、`ViewRenderer` の API も合わせて変更が必要（`&SimState` → `&HashMap<Pos, bool>` または `WireSimulator` から直接取得）。

## trait 不要の判断

当初、旧実装と新実装の共存のために `SimulatorEngine` trait を導入する案があったが、以下の理由で不要と判断した:

- 旧実装は完全に削除するため、複数実装が同時に存在しない
- `WasmSimulator` は `wasm_bindgen` の制約上、ジェネリクスで export できないが、具体型 `WireSimulator` を直接保持すれば問題ない
- trait 抽象化は将来の拡張時に追加できる（YAGNI）
