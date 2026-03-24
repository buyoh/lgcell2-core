# シミュレーションモデル

伝搬ルール、遅延の決定方法、ステップ実行エンジンの仕組みを解説する。

## 伝搬ルールの概要

LGCELL2 のシミュレーションは **tick** を単位として進行する。各 tick では、全セルを座標の辞書順 `(x, y)` に 1 つずつ処理する。

### ワイヤの遅延

ワイヤが即時伝搬か遅延伝搬かは、`src` と `dst` の座標順序で **自動的に** 決まる。明示的な遅延パラメータは不要。

| 条件 | 遅延 | 参照する値 | 理由 |
|------|------|-----------|------|
| `dst < src`（辞書順） | 1 tick 遅延 | `prev_state[src]` | dst が先に処理されるため、src の値はまだ更新されていない。前 tick の値を使う |
| `dst >= src`（辞書順） | 即時 | `curr_state[src]` | src が先に処理済みのため、今 tick の計算結果を使える |

### 振動の排除

- **self-loop 禁止**: `src == dst` のワイヤは構築時にエラーとなる
- **即時伝搬のサブグラフは DAG**: 即時伝搬のみで構成されるサブグラフでは、ワイヤが常に辞書順で前方を向くため、サイクルが生じ得ない
- **フィードバックには必ず遅延が入る**: 辞書順で逆行する辺には 1 tick の遅延が入るため、同一 tick 内での振動は原理的に発生しない

### セル値の計算

```
step(cell):
  incoming_wires = circuit.incoming[cell]

  if incoming_wires is empty:
    curr_state[cell] = prev_state[cell]   // 値を保持
  else:
    values = []
    for wire in incoming_wires:
      if wire.dst < wire.src:
        src_val = prev_state[wire.src]     // 遅延伝搬
      else:
        src_val = curr_state[wire.src]     // 即時伝搬
      values.push(wire.propagate(src_val))
    curr_state[cell] = max(values)         // OR 合成
```

## Simulator の構造

```rust
pub struct Simulator {
    circuit: Circuit,
    prev_state: SimState,    // 前 tick の確定状態
    curr_state: SimState,    // 現在 tick の計算中状態
    tick: u64,               // 現在の tick 番号（0-origin）
    cell_index: usize,       // 次に処理するセルのインデックス
}
```

`prev_state` と `curr_state` の 2 つの状態を保持することで、遅延伝搬と即時伝搬を区別する。tick 完了時に `curr_state` を `prev_state` にコピーして次の tick に備える。

## ステップ実行と中断・再開

Web 上での利用を想定し、シミュレーションはセル 1 つ分の粒度で中断・再開できる。

### API の粒度

| メソッド | 粒度 | 用途 |
|----------|------|------|
| `step()` | セル 1 個 | 中断ポイント。`StepResult::Continue` または `TickComplete` を返す |
| `tick()` | 1 tick | 1 tick 分の全セルをまとめて処理 |
| `run(n)` | n tick | 指定 tick 数を一括実行 |
| `run_with_snapshots(n)` | n tick | 各 tick の `TickSnapshot` を収集して返す |

### StepResult

```rust
pub enum StepResult {
    Continue,      // tick 内に未処理セルが残っている
    TickComplete,  // tick の全セル処理完了
}
```

`step()` を呼び続ければ `tick()` と同じ結果になることが保証されている。途中で中断しても、次回の `step()` 呼び出しで正確に再開できる。

## 状態の外部操作

入力セルの値を設定するために `state_mut()` が提供される。

```rust
pub fn state_mut(&mut self) -> StateMut<'_>
```

`StateMut::set(pos, value)` は `prev_state` と `curr_state` の両方を同時に更新する。これにより、tick 開始前にどちらの状態を参照されても正しい値が読まれることが保証される。

## tick ライフサイクル

```
              tick 0 開始
                 │
    ┌────────────┼────────────┐
    │  cell_index = 0         │
    │  step(): cell[0] 処理   │
    │  step(): cell[1] 処理   │
    │  ...                    │
    │  step(): 最後のセル処理 │
    │  → TickComplete         │
    │                         │
    │  prev_state = curr_state│
    │  cell_index = 0         │
    │  tick += 1              │
    └────────────┼────────────┘
                 │
              tick 1 開始
                 │
                ...
```

## 伝搬の具体例

### 即時伝搬チェーン

```
(0,0) →Pos→ (1,0) →Pos→ (2,0)
```

処理順: `(0,0)` → `(1,0)` → `(2,0)`

- `(0,0)` の値を `(1,0)` に伝搬（即時: `dst=(1,0) >= src=(0,0)`）
- `(1,0)` の計算結果を `(2,0)` に伝搬（即時: `dst=(2,0) >= src=(1,0)`）
- **1 tick で全て伝搬完了**

### 遅延伝搬（逆方向ワイヤ）

```
(1,0) →Pos→ (0,0)
```

処理順: `(0,0)` → `(1,0)`

- `(0,0)` 処理時、`src=(1,0)` はまだ未処理 → `dst=(0,0) < src=(1,0)` → 遅延伝搬
- **前 tick の `(1,0)` の値を使用**

### フィードバックループ

```
(0,0) →Neg→ (1,0) →Neg→ (0,0)
```

- `(0,0) → (1,0)`: 即時伝搬（`dst >= src`）
- `(1,0) → (0,0)`: 遅延伝搬（`dst < src`）

フィードバックには必ず 1 tick の遅延が含まれるため、同一 tick 内での振動は発生しない。
