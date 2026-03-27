# シミュレーションエンジンの変更

## 概要

階層的シミュレーション方式を採用する。各モジュールインスタンスは独自の `Simulator` を保持し、親の tick 処理中に適切なタイミングで 1 tick 分実行される。

## Simulator の変更

### 構造体の拡張

```rust
pub struct Simulator {
    circuit: Circuit,
    prev_state: SimState,
    curr_state: SimState,
    sub_simulators: Vec<Simulator>,  // 新規: モジュールごとの子 Simulator
    tick: u64,
    cell_index: usize,
}
```

### 事前計算データ

`Simulator::new()` で以下のルックアップテーブルを構築する:

```rust
/// モジュール出力セルの集合。step() で通常処理をスキップするために使用。
module_output_cells: HashSet<Pos>,

/// 最初の出力セル → モジュールインデックス。サブ回路評価のトリガーに使用。
module_triggers: HashMap<Pos, usize>,
```

`module_triggers` は各モジュールの `output` 配列の最初の要素をキーとする。

### step() の変更

```
step():
    cell = sorted_cells[cell_index]

    // 1. モジュール評価トリガーチェック
    if cell in module_triggers:
        module_index = module_triggers[cell]
        evaluate_module(module_index)

    // 2. セル値の計算
    if cell in module_output_cells:
        // モジュール出力セル: evaluate_module で設定済み。何もしない。
        // (prev_state からの値保持も不要)
    else:
        // 通常のセル処理 (既存ロジックそのまま)
        incoming = circuit.incoming_indices(cell)
        if incoming.is_empty():
            curr_state[cell] = prev_state[cell]
        else:
            // OR 合成 (既存)
            ...

    // 3. インデックス更新 (既存ロジック)
    cell_index += 1
    if cell_index >= sorted_cells.len():
        // tick 完了処理
        prev_state = curr_state.clone()
        cell_index = 0
        tick += 1
        return TickComplete
    else:
        return Continue
```

### evaluate_module() の実装

```
evaluate_module(module_index):
    module = circuit.modules()[module_index]
    sub_sim = sub_simulators[module_index]

    // 1. 親の入力値をサブ回路に注入
    for i in 0..module.input.len():
        parent_value = curr_state[module.input[i]]
        sub_sim.state_mut().set(module.sub_input[i], parent_value)

    // 2. サブ回路を 1 tick 実行
    sub_sim.tick()

    // 3. サブ回路の出力値を親に反映
    for j in 0..module.output.len():
        sub_value = sub_sim.prev_state.get(module.sub_output[j])
        curr_state.set(module.output[j], sub_value)
```

## タイミング詳細

### 組合せ回路としてのサブ回路

サブ回路内の全ワイヤが前方向（`src < dst`）の場合、1 tick で入力から出力まで伝搬が完了する。親の 1 tick 内でサブ回路の結果が確定し、後続の親セルから即座に参照できる。

ポート列制約により、入力列（x=xi）が全て処理された後に出力列（x=xo）に到達することが保証されるため、入力値は常に確定済みである。

```
親 tick N:
  (0,0) 処理 → 値確定
  (1,0) 処理 → (0,0) からの Neg ワイヤ → 値確定 [モジュール入力列 x=1]
  (1,1) 処理 → (0,0) からの Neg ワイヤ → 値確定 [モジュール入力列 x=1]
  (2,0) 到達 → モジュールトリガー [モジュール出力列 x=2]
    サブ回路 tick: (1,0),(1,1) の値をサブ入力に注入 → サブ回路内で伝搬 → 出力確定
    (2,0) に出力値を設定 [モジュール出力]
    (2,1) に出力値を設定 [モジュール出力]
  (2,1) 到達 → モジュール出力セル → スキップ（設定済み）
  (3,0) 処理 → (2,0) からのワイヤ → 値確定（サブ回路の結果を即時参照）
```

### フラット展開との等価性

ポート列制約は、サブ回路がフラット展開された場合（サブ回路の内部セルが x_input < x < x_output の範囲を占める）と等価な動作を保証する:

1. 入力列の全セルが確定した後にサブ回路を評価 → フラット展開で入力セルが先に処理されるのと同じ
2. サブ回路内は独自座標系で辞書順処理 → フラット展開での内部セル処理順と一致
3. 出力列に結果を書き込み、後続セルから即時参照可能 → フラット展開での出力セル処理と同じ

### 順序回路としてのサブ回路

サブ回路にフィードバック（逆方向ワイヤ）が含まれる場合、サブ回路の内部状態は tick 間で保持される。親の各 tick でサブ回路も 1 tick 進むため、サブ回路内のフリップフロップ等が正しく動作する。

```
親 tick N:   サブ回路 tick N → 内部状態を prev_state に保存
親 tick N+1: サブ回路 tick N+1 → 前 tick の内部状態を参照可能
```

## state_mut() との相互作用

`Simulator::state_mut()` は親回路のセルのみ更新可能。サブ回路の内部セルは外部から直接操作できない。モジュール入力セルを `state_mut()` で設定すれば、次の tick でサブ回路に反映される。

## TickSnapshot の拡張

初期バージョンでは `TickSnapshot` に変更を加えない。スナップショットには親回路のセルのみ含まれる。モジュール出力セルの値は含まれるため、サブ回路の計算結果は親回路を通じて観測可能。

将来の拡張として、サブ回路の内部状態を含む `ModuleSnapshot` の追加を検討する:

```rust
// 将来拡張
pub struct ModuleSnapshot {
    pub module_index: usize,
    pub snapshot: TickSnapshot,
}
```

## ステップ実行の粒度

`step()` メソッドはセル 1 個分の粒度を維持する。モジュール出力セルに到達した際にサブ回路の全 tick を内部で実行するが、これは 1 回の `step()` 呼び出し内で完了する。サブ回路のステップ実行が必要な場合は、将来の拡張として `step_into()` 等のメソッドを検討する。
