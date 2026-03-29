# シミュレーションモデル

伝搬ルール、遅延の決定方法、ステップ実行エンジンの仕組みを解説する。

## 伝搬ルールの概要

LGCELL2 のシミュレーションは **tick** を単位として進行する。各 tick では、全セルを座標の辞書順 `(x, y)` に 1 つずつ処理する。

### ワイヤの遅延

ワイヤが即時伝搬か遅延伝搬かは、`src` と `dst` の座標順序で **自動的に** 決まる。明示的な遅延パラメータは不要。

| 条件 | 遅延 | 参照する値 | 理由 |
|------|------|-----------|------|
| `dst < src`（辞書順） | 1 tick 遅延 | `wire_state` の遅延スロット | dst が先に処理されるため、src の値はまだ更新されていない。前 tick 完了時に保存した値を使う |
| `dst >= src`（辞書順） | 即時 | `cell_values[src]` | src が先に処理済みのため、今 tick の計算結果を直接参照できる |

### 振動の排除

- **self-loop 禁止**: `src == dst` のワイヤは構築時にエラーとなる
- **即時伝搬のサブグラフは DAG**: 即時伝搬のみで構成されるサブグラフでは、ワイヤが常に辞書順で前方を向くため、サイクルが生じ得ない
- **フィードバックには必ず遅延が入る**: 辞書順で逆行する辺には 1 tick の遅延が入るため、同一 tick 内での振動は原理的に発生しない

### セル値の計算

```
step(cell):
  incoming_wires = circuit.incoming[cell]

  if incoming_wires is empty:
    cell_values[cell] = wire_state.get_stateless_cell(cell)  // 前 tick の値を保持
  else:
    values = []
    for wire in incoming_wires:
      if wire.dst < wire.src:
        src_val = wire_state.get_delayed_wire(wire)           // 遅延伝搬
      else:
        src_val = cell_values[wire.src]                       // 即時伝搬
      values.push(wire.propagate(src_val))
    cell_values[cell] = max(values)                           // OR 合成
```

## WireSimulator の構造

### 概要

`WireSimulator` は遅延ワイヤベースの中断可能シミュレーションエンジンである。旧 `Simulator` が `prev_state` / `curr_state` の 2 つの `HashMap<Pos, bool>` を保持し毎 tick クローンしていたのに対し、`WireSimulator` は遅延が必要な値のみを `WireSimState` に保存し、`cell_values` を in-place で更新する。

```rust
pub struct WireSimulator {
    circuit: Circuit,
    wire_state: WireSimState,          // 遅延ワイヤ・入力なしセルの前 tick 値
    cell_values: Vec<bool>,            // 全セルの現在値（sorted_cells と同順）
    cell_pos_to_index: HashMap<Pos, usize>, // Pos → インデックスの逆引き
    tick: u64,                         // 現在の tick 番号（0-origin）
    cell_index: usize,                 // 次に処理するセルのインデックス
    output_format: OutputFormat,       // tick 完了時の出力形式
}
```

### WireSimState — 遅延値ストア

遅延伝搬に必要な値のみを `Vec<bool>` のスロットで管理する。全セルの状態をコピーする必要がなく、メモリ効率とコピーコストの両面で有利。

```rust
pub struct WireSimState {
    delayed_values: Vec<bool>,                // 遅延スロットの値
    wire_to_slot: HashMap<usize, usize>,      // 遅延ワイヤのインデックス → スロット
    cell_to_slot: HashMap<usize, usize>,      // 入力なしセルのインデックス → スロット
}
```

**スロットの割り当て対象:**
- **遅延ワイヤ** (`dst < src`): 前 tick の `src` 値を保存するためのスロット
- **入力なしセル**: 入力ワイヤも `InputComponent` もないセルは、前 tick の値を次 tick に引き継ぐためのスロット

初期化 (`from_circuit`) で回路を走査し、上記の条件に該当する要素にのみスロットを割り当てる。即時伝搬ワイヤや入力付きセルにはスロットは不要。

## ステップ実行と中断・再開

Web 上での利用を想定し、シミュレーションはセル 1 つ分の粒度で中断・再開できる。

### API の粒度

| メソッド | 粒度 | 用途 |
|----------|------|------|
| `step()` | セル 1 個 | 中断ポイント。`StepResult::Continue` または `TickComplete` を返す |
| `tick()` | 1 tick | 1 tick 分の全セルをまとめて処理 |
| `run(n)` | n tick | 指定 tick 数を一括実行 |
| `run_with_snapshots(n)` | n tick | 各 tick の `TickOutput` を収集して返す |
| `run_with_verification(n)` | n tick | 各 tick のテスター検証結果を収集して返す |

### StepResult

```rust
pub enum StepResult {
    Continue,      // tick 内に未処理セルが残っている
    TickComplete,  // tick の全セル処理完了
}
```

`step()` を呼び続ければ `tick()` と同じ結果になることが保証されている。途中で中断しても、次回の `step()` 呼び出しで正確に再開できる。

## 状態の外部操作

セルの値を外部から読み書きするために、`get_cell()` と `set_cell()` が提供される。

```rust
pub fn get_cell(&self, pos: Pos) -> Option<bool>
pub fn set_cell(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError>
```

`set_cell()` は `cell_values` の更新に加え、関連する遅延スロット（`wire_state` 内のワイヤスロット・セルスロット）も同時に更新する。これにより、次の tick でどのタイミングで参照されても正しい値が読まれることが保証される。

また、全セルの現在値を `HashMap<Pos, bool>` で一括取得する `cell_values()` も提供される。

## 出力形式 (OutputFormat)

`run_with_snapshots()` が返す `TickOutput` に含めるセルの範囲を `OutputFormat` で制御できる。

```rust
pub enum OutputFormat {
    AllCell,              // すべてのセルの状態を収集
    ViewPort(Vec<Rect>),  // 指定矩形領域内のセルのみ収集
}
```

`ViewPort` は描画対象領域のみを返すため、大規模回路で出力データ量を削減できる。`set_output_format()` で実行中に変更可能。

## 入力コンポーネントとテスター

### InputComponent

`InputComponent` は tick 番号に応じた入力値を自動的に供給するコンポーネントである。各 tick の開始時に `apply_inputs()` で全入力コンポーネントの値が対象セルに設定される。

### テスター検証

`verify_testers()` は直近の tick 完了後のセル値をテスター定義と照合し、不一致を `TesterResult` として返す。`run_with_verification(n)` は n tick 実行しながらテスター検証結果を蓄積する。

```rust
pub struct TesterResult {
    pub target: Pos,
    pub tick: u64,
    pub expected: bool,
    pub actual: bool,
}
```

## tick ライフサイクル

```
              tick 0 開始
                 │
    ┌────────────┼────────────┐
    │  apply_inputs()         │  ← InputComponent の値を設定
    │  cell_index = 0         │
    │  step(): cell[0] 処理   │
    │  step(): cell[1] 処理   │
    │  ...                    │
    │  step(): 最後のセル処理 │
    │  → TickComplete         │
    │                         │
    │  complete_tick():        │
    │    遅延ワイヤの値を      │
    │    wire_state に保存     │
    │    入力なしセルの値を    │
    │    wire_state に保存     │
    │    cell_index = 0       │
    │    tick += 1            │
    └────────────┼────────────┘
                 │
              tick 1 開始
                 │
                ...
```

### tick 完了時の処理 (complete_tick)

tick の全セル処理後、次 tick の遅延伝搬に必要な値を `wire_state` に保存する:

1. **遅延ワイヤの更新**: `dst < src` の各ワイヤについて、`cell_values[src]` の値を `wire_state` のスロットに書き込む
2. **入力なしセルの更新**: 入力ワイヤも `InputComponent` もないセルの `cell_values` を `wire_state` のスロットに書き込む

これにより、次 tick で `get_delayed_wire()` や `get_stateless_cell()` が最新の遅延値を返せるようになる。

## 伝搬の具体例

### 即時伝搬チェーン

```
(0,0) →Pos→ (1,0) →Pos→ (2,0)
```

処理順: `(0,0)` → `(1,0)` → `(2,0)`

- `(0,0)` の値を `(1,0)` に伝搬（即時: `dst=(1,0) >= src=(0,0)` → `cell_values[src]` を参照）
- `(1,0)` の計算結果を `(2,0)` に伝搬（即時: `dst=(2,0) >= src=(1,0)` → `cell_values[src]` を参照）
- **1 tick で全て伝搬完了**

### 遅延伝搬（逆方向ワイヤ）

```
(1,0) →Pos→ (0,0)
```

処理順: `(0,0)` → `(1,0)`

- `(0,0)` 処理時、`src=(1,0)` はまだ未処理 → `dst=(0,0) < src=(1,0)` → 遅延伝搬
- **`wire_state` のスロットに保存された前 tick の `(1,0)` の値を使用**

### フィードバックループ

```
(0,0) →Neg→ (1,0) →Neg→ (0,0)
```

- `(0,0) → (1,0)`: 即時伝搬（`dst >= src`）
- `(1,0) → (0,0)`: 遅延伝搬（`dst < src`）

フィードバックには必ず 1 tick の遅延が含まれるため、同一 tick 内での振動は発生しない。
