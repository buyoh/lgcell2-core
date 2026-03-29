# シミュレーションエンジンの変更

## 概要

階層的シミュレーション方式を採用する。各モジュールインスタンスは独自の `Simulator` を保持し、親の tick 処理中に適切なタイミングで 1 tick 分実行される。

## 現行 Simulator の構造

ワイヤ状態モデルへの移行（2026-03-28 完了）により、シミュレータは以下の構造を持つ:

```rust
pub struct Simulator {
    circuit: Circuit,
    wire_state: WireSimState,              // 遅延ワイヤ・入力なしセルの前 tick 値
    cell_values: Vec<bool>,                // 全セルの現在値（sorted_cells と同順）
    cell_pos_to_index: HashMap<Pos, usize>, // Pos → インデックスの逆引き
    tick: u64,
    cell_index: usize,
    last_output: TickOutput,               // 直近 tick の出力キャッシュ
    output_format: OutputFormat,           // 出力形式（AllCell / ViewPort）
}
```

遅延伝搬が必要なワイヤと入力なしセルの値のみを `WireSimState` のスロットで管理し、`cell_values` を in-place で更新する。tick 完了時に `complete_tick()` で遅延スロットを更新する（全セルのクローンは不要）。

## Simulator の拡張

### 構造体の変更

```rust
pub struct Simulator {
    circuit: Circuit,
    wire_state: WireSimState,
    cell_values: Vec<bool>,
    cell_pos_to_index: HashMap<Pos, usize>,
    tick: u64,
    cell_index: usize,
    last_output: TickOutput,
    output_format: OutputFormat,
    // --- 新規 ---
    sub_simulators: Vec<Simulator>,        // モジュールごとの子 Simulator
    module_output_cells: HashSet<usize>,   // モジュール出力セルのインデックス集合
    module_triggers: HashMap<usize, usize>, // 最初の出力セルインデックス → モジュールインデックス
}
```

### 事前計算データ

`Simulator::new()` で以下のルックアップテーブルを構築する:

```rust
/// モジュール出力セルのインデックス集合。step() で通常処理をスキップするために使用。
module_output_cells: HashSet<usize>,

/// 最初の出力セルインデックス → モジュールインデックス。サブ回路評価のトリガーに使用。
module_triggers: HashMap<usize, usize>,
```

`module_triggers` は各モジュールの `output` 配列の最初の要素（辞書順最小）を `cell_pos_to_index` で変換したインデックスをキーとする。

### step() の変更

```
step():
    if cell_index == 0:
        apply_inputs()

    cell_idx = cell_index
    cell = sorted_cells[cell_idx]

    // 1. モジュール評価トリガーチェック
    if cell_idx in module_triggers:
        module_index = module_triggers[cell_idx]
        evaluate_module(module_index)

    // 2. セル値の計算
    if cell_idx in module_output_cells:
        // モジュール出力セル: evaluate_module で設定済み。何もしない。
    else:
        // 通常のセル処理（既存ロジックそのまま）
        incoming = circuit.incoming_indices(cell)
        if incoming.is_empty():
            if wire_state.get_stateless_cell(cell_idx) is Some(val):
                cell_values[cell_idx] = val
        else:
            next_value = false
            for wire_idx in incoming:
                wire = circuit.wires()[wire_idx]
                if wire.dst < wire.src:
                    src_val = wire_state.get_delayed_wire(wire_idx)
                else:
                    src_idx = cell_pos_to_index[wire.src]
                    src_val = cell_values[src_idx]
                next_value = next_value || wire.propagate(src_val)
                if next_value: break  // OR 短絡
            cell_values[cell_idx] = next_value

    // 3. インデックス更新（既存ロジック）
    cell_index += 1
    if cell_index >= sorted_cells.len():
        complete_tick()  // 遅延スロット更新、last_output 再構築、tick++
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
    //    set_cell() は cell_values と wire_state の両方を更新するため、
    //    サブ回路の tick 開始時に正しい値が読み取られる。
    for i in 0..module.input.len():
        parent_idx = cell_pos_to_index[module.input[i]]
        parent_value = cell_values[parent_idx]
        sub_sim.set_cell(module.sub_input[i], parent_value)

    // 2. サブ回路を 1 tick 実行
    //    tick() 内部で apply_inputs() → 全セル step → complete_tick() が行われる。
    //    サブ回路には InputComponent がないため apply_inputs() は no-op。
    //    sub_input セルは入力ワイヤなし → wire_state から値を復元（set_cell で設定済み）。
    sub_sim.tick()

    // 3. サブ回路の出力値を親に反映
    //    tick() 完了後、cell_values にはそのtickの計算結果が残っている。
    for j in 0..module.output.len():
        sub_value = sub_sim.get_cell(module.sub_output[j])
        parent_idx = cell_pos_to_index[module.output[j]]
        cell_values[parent_idx] = sub_value
```

### 入力値注入の仕組み

サブ回路の `sub_input` セルは以下の性質を持つ:

- InputComponent（Generator）を持たない（サブ回路は Generator/Tester を含まない）
- 入力ワイヤを持たない（制約 9: sub_input にワイヤの dst は不可）
- したがって `WireSimState` の「入力なしセル」として遅延スロットが割り当てられる

`set_cell()` は `cell_values[index]` と `wire_state` の関連スロットの両方を更新する。その後 `tick()` が呼ばれると、`sub_input` セルの処理時に `incoming.is_empty()` → `wire_state.get_stateless_cell()` から `set_cell()` で設定した値が読み取られ、正しく伝搬される。

### complete_tick() との相互作用

サブ回路の `tick()` 内部で `complete_tick()` が実行され、サブ回路自身の `wire_state` が更新される。親の `complete_tick()` はサブ回路の状態に関与しない。

親の `complete_tick()` ではモジュール出力セルの値が `wire_state` に保存される（モジュール出力セルを src とする遅延ワイヤがある場合、またはモジュール出力セルが別のモジュールの入力として使われている場合）。

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
    set_cell() で (1,0),(1,1) の値をサブ入力に注入
    サブ回路 tick: サブ回路内で伝搬 → 出力確定
    get_cell() で出力値を取得 → cell_values[(2,0)], cell_values[(2,1)] に設定
  (2,1) 到達 → モジュール出力セル → スキップ（設定済み）
  (3,0) 処理 → (2,0) からのワイヤ → 値確定（cell_values から即時参照）
```

### フラット展開との等価性

ポート列制約は、サブ回路がフラット展開された場合（サブ回路の内部セルが x_input < x < x_output の範囲を占める）と等価な動作を保証する:

1. 入力列の全セルが確定した後にサブ回路を評価 → フラット展開で入力セルが先に処理されるのと同じ
2. サブ回路内は独自座標系で辞書順処理 → フラット展開での内部セル処理順と一致
3. 出力列に結果を書き込み、後続セルから即時参照可能 → フラット展開での出力セル処理と同じ

### 順序回路としてのサブ回路

サブ回路にフィードバック（逆方向ワイヤ）が含まれる場合、サブ回路の内部状態は tick 間で保持される。親の各 tick でサブ回路も 1 tick 進むため、サブ回路内のフリップフロップ等が正しく動作する。

```
親 tick N:   サブ回路 tick N → 内部状態を wire_state のスロットに保存
親 tick N+1: サブ回路 tick N+1 → wire_state から前 tick の遅延値を参照
```

## set_cell() との相互作用

`Simulator::set_cell()` は親回路のセルのみ更新可能。サブ回路の内部セルは外部から直接操作できない。モジュール入力セルを `set_cell()` で設定すれば、次の tick の `evaluate_module()` でサブ回路に反映される。

`set_cell()` は `cell_values` の更新に加え、`wire_state` の関連スロット（遅延ワイヤ・入力なしセル）も同時に更新する。さらに `replay_tick()` を呼んで `last_output` キャッシュも即座に再構築する。

## TickOutput の拡張

初期バージョンでは `TickOutput` に変更を加えない。出力には親回路のセルのみ含まれる。モジュール出力セルの値は含まれるため、サブ回路の計算結果は親回路を通じて観測可能。

将来の拡張として、サブ回路の内部状態を含む `ModuleOutput` の追加を検討する:

```rust
// 将来拡張
pub struct ModuleOutput {
    pub module_index: usize,
    pub output: TickOutput,
}
```

## ステップ実行の粒度

`step()` メソッドはセル 1 つ分の粒度を維持する。モジュール出力セルに到達した際にサブ回路の全 tick を内部で実行するが、これは 1 回の `step()` 呼び出し内で完了する。サブ回路のステップ実行が必要な場合は、将来の拡張として `step_into()` 等のメソッドを検討する。
