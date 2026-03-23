# シミュレーションエンジン設計

中断可能なステップ実行エンジンと伝搬ルールを設計する。

作成日: 2026-03-23
ステータス: 実装完了

## 背景・動機

- Web 上での利用を想定し、1 tick の計算途中でも中断・再開できる必要がある。
- 大規模回路でも 1 tick がブロッキングしないよう、セル単位でのステップ実行を可能にする。
- async/await や unstable な generator を使わず、明示的なステートマシンとして実装する。

## 設計・方針

### 伝搬ルール

セルは (x, y) 辞書順に処理される。ワイヤの伝搬遅延は src と dst の順序関係で決まる。

```
dst < src  (辞書順)  →  1 tick 遅延（前の tick の src 値を使用）
dst >= src (辞書順)  →  即時伝搬（現在の tick の src 値を使用）
```

- 「即時」とは、同一 tick 内で src が先に処理済みであるため、その結果を使えるということ。
- 順序が逆行するワイヤには必ず 1 tick の遅延が入るため、**同一 tick 内での振動は原理的に発生しない**。即時伝搬のみで構成されるサブグラフは DAG になる（フィードバックには必ず遅延辺が含まれる）。

### SimState — シミュレーション状態

```rust
/// 各セルの現在値を保持する。
pub struct SimState {
    values: HashMap<Pos, bool>,
}
```

- `Circuit` の `cells` と同じキーセットを持つ。
- tick ごとにスナップショットとして保持できるよう `Clone` を実装する。

### StepResult — ステップの結果

```rust
/// `Simulator::step()` の戻り値。
pub enum StepResult {
    /// 1 セル処理完了。現在の tick にまだ未処理セルがある。
    Continue,
    /// 現在の tick の全セル処理完了。
    TickComplete,
}
```

### Simulator — ステップ実行エンジン

```rust
/// 中断可能なシミュレーションエンジン。
pub struct Simulator {
    circuit: Circuit,
    /// 前の tick の状態。遅延ワイヤの参照用。
    prev_state: SimState,
    /// 現在の tick で計算中の状態。
    curr_state: SimState,
    /// 現在の tick 番号 (0-origin)。
    tick: u64,
    /// 現在の tick 内で次に処理すべきセルのインデックス。
    cell_index: usize,
}
```

### ステップ実行のアルゴリズム

```
step():
  1. cell = circuit.sorted_cells[cell_index]
  2. incoming_wires = circuit.incoming[cell]
  3. if incoming_wires is empty:
       curr_state[cell] = prev_state[cell]  // 初期値保持
     else:
       values = []
       for wire in incoming_wires:
         src_value = if wire.dst < wire.src:
                       prev_state[wire.src]   // 遅延伝搬
                     else:
                       curr_state[wire.src]   // 即時伝搬
         propagated = match wire.kind:
                        Positive => src_value
                        Negative => 1 - src_value
         values.push(propagated)
       curr_state[cell] = max(values)
  4. cell_index += 1
  5. if cell_index >= circuit.sorted_cells.len():
       prev_state = curr_state.clone()
       cell_index = 0
       tick += 1
       return TickComplete
     else:
       return Continue
```

### 高レベル API

```rust
impl Simulator {
    /// 新しいシミュレータを構築する。
    pub fn new(circuit: Circuit) -> Self;

    /// 1 セル分だけ進める。中断ポイント。
    pub fn step(&mut self) -> StepResult;

    /// 1 tick 完了まで進める。
    pub fn tick(&mut self) -> &SimState;

    /// 指定 tick 数だけ進める。
    pub fn run(&mut self, ticks: u64) -> &SimState;

    /// 現在の状態を取得する。
    pub fn state(&self) -> &SimState;

    /// 現在の tick 番号を取得する。
    pub fn current_tick(&self) -> u64;
}
```

> **TODO（微分可能モード）:** 将来的に `bool` を実数型に拡張し、smooth max（LogSumExp 等）による勾配計算を導入予定。高難易度のため現段階では設計・実装しない。

### テスト方針

- **単一セル・単一ワイヤ**: Positive/Negative それぞれの伝搬を確認。
- **即時伝搬チェーン**: A→B→C（すべて前方）で 1 tick で伝搬することを確認。
- **遅延伝搬**: A→B（後方ワイヤ）で 1 tick 遅れることを確認。
- **フィードバックループ**: A→B (即時), B→A (遅延) のループで振動せず安定することを確認。
- **複数入力 OR**: 2 本のワイヤの max が取られることを確認。
- **NAND ゲート**: Negative ワイヤ 2 本で NAND 動作を確認。
- **半加算器**: S = XOR(A,B), C = AND(A,B) の組合せ回路で正しい出力を確認。
- **ステップ中断・再開**: `step()` を途中で止めて再開しても結果が変わらないことを確認。
