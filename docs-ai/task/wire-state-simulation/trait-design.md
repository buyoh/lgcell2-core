# SimulatorEngine trait 設計

旧実装（`Simulator`）と新実装（`WireSimulator`）の共通インターフェースを定義する。

## trait 定義

```rust
use crate::base::SimulationError;
use crate::circuit::{Circuit, Pos};

/// シミュレーションエンジンの共通インターフェース。
pub trait SimulatorEngine {
    /// 1 セル分だけ進める。中断ポイント。
    fn step(&mut self) -> StepResult;

    /// 1 tick 完了まで進める。
    fn tick(&mut self);

    /// 指定 tick 数だけ進める。
    fn run(&mut self, ticks: u64);

    /// 指定 tick 数だけ進め、各 tick の状態をスナップショットとして返す。
    fn run_with_snapshots(&mut self, ticks: u64) -> Vec<TickSnapshot>;

    /// 直近で完了した tick のテスター検証を行い、不一致を返す。
    fn verify_testers(&self) -> Vec<TesterResult>;

    /// 指定 tick 数だけ進め、各 tick のテスター検証結果を収集して返す。
    fn run_with_verification(&mut self, ticks: u64) -> Vec<TesterResult>;

    /// 回路定義を取得する。
    fn circuit(&self) -> &Circuit;

    /// 指定セルの値を取得する。
    fn get_cell(&self, pos: Pos) -> Option<bool>;

    /// 指定セルの値を設定する（入力注入用）。
    fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError>;

    /// 全セルの値を辞書順で返す。
    fn cell_values(&self) -> Vec<(Pos, bool)>;

    /// 現在の tick 番号を取得する。
    fn current_tick(&self) -> u64;

    /// 現在 tick 内で処理対象のセルを返す。
    fn current_cell(&self) -> Option<Pos>;
}
```

## 設計判断

### `state()` / `state_mut()` を trait に含めない理由

現行の `state()` は `&SimState` を返し、`state_mut()` は `StateMut<'_>` を返す。これらは `Simulator` 固有の型であり、`WireSimulator` は異なる内部表現を持つため、trait のメソッドとしては不適切。

代替として:

| 現行 API | trait メソッド | 説明 |
|----------|---------------|------|
| `state().get(pos)` | `get_cell(pos)` | セル値の取得 |
| `state_mut().set(pos, value)` | `set_cell(pos, value)` | セル値の設定 |
| `state().values()` | `cell_values()` | 全セル値の一覧 |

### `tick()` / `run()` の戻り値

現行の `tick()` / `run()` は `&SimState` を返すが、trait では戻り値を `()` に変更する。状態の取得は `get_cell()` / `cell_values()` で別途行う。理由:

- 旧実装: `&self.prev_state` を直接返せる
- 新実装: `SimState` を持たないため、返却のために一時オブジェクトを構築する必要がある
- 利用側は `tick()` 後に `get_cell()` で必要な値を取得すれば十分

### デフォルト実装

`tick()`, `run()`, `run_with_snapshots()`, `run_with_verification()` は `step()` の繰り返しで実装できるため、trait にデフォルト実装を提供する:

```rust
impl dyn SimulatorEngine {
    // デフォルト実装は trait 定義内に記述
}
```

ただし、 `run()` 等のデフォルト実装では `cell_values()` を毎 tick 呼ぶオーバーヘッドが生じうるため、各実装で最適化されたオーバーライドを許可する。

## 旧 `Simulator` への trait 実装

`Simulator` に `SimulatorEngine` を実装する。既存のメソッドはそのまま残し、trait メソッドは既存メソッドへの委譲で実装する:

```rust
impl SimulatorEngine for Simulator {
    fn step(&mut self) -> StepResult {
        self.step()  // 既存メソッド
    }

    fn get_cell(&self, pos: Pos) -> Option<bool> {
        self.state().get(pos)
    }

    fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
        self.state_mut().set(pos, value)
    }

    fn cell_values(&self) -> Vec<(Pos, bool)> {
        self.circuit()
            .sorted_cells()
            .iter()
            .map(|&pos| (pos, self.state().get(pos).unwrap()))
            .collect()
    }

    // ... 他は同様に委譲
}
```

**注意**: `step()` のメソッド名衝突を避けるため、既存メソッドは固有メソッドとして残し、trait 実装は `SimulatorEngine::step()` として呼び出す。Rust では固有メソッドが優先されるため、既存コードへの影響はない。

## 利用側の移行

trait オブジェクト `Box<dyn SimulatorEngine>` または ジェネリクス `T: SimulatorEngine` で利用側を抽象化する:

```rust
// WasmSimulator
pub struct WasmSimulator {
    simulator: Box<dyn SimulatorEngine>,
}

// または型パラメータ
pub struct WasmSimulator<E: SimulatorEngine> {
    simulator: E,
}
```

wasm_bindgen の制約上、trait オブジェクト方式が適切と考えられる（ジェネリクスは wasm_bindgen で export できないため）。
