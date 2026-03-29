# ワイヤ状態モデルによるシミュレーション再設計

シミュレーション状態の保持単位を「セル」から「遅延ワイヤ」に変更し、概念の明確化とメモリ効率を改善する。旧実装（`Simulator`）は `WireSimulator` で完全に置き換える。

作成日: 2026-03-28
ステータス: 設計中

## 背景・動機

現行の `Simulator` はセルごとに `bool` 値を保持する `SimState`（`HashMap<Pos, bool>`）を 2 つ（`prev_state`, `curr_state`）持ち、tick 完了時に `curr_state` を `prev_state` にクローンする設計である。

しかし現在の仕様では、セルは辞書順 `(x, y)` で処理され:

- **辞書順前方ワイヤ** (`dst >= src`): 即時伝搬。`curr_state[src]` を参照するため、値は処理時点で確定済み
- **辞書順後方ワイヤ** (`dst < src`): 遅延伝搬。`prev_state[src]` を参照し、1 tick 遅れた値を使う

つまり、**tick 間で持ち越す情報は遅延ワイヤの値だけ**であり、前方ワイヤの値は tick 中に即座に決定される。全セルの状態を 2 重保持するのは冗長である。

状態を「遅延ワイヤが保持するもの」と再解釈することで:

1. **概念の明確化**: 状態の所在が遅延辺に限定され、モデルが簡潔になる
2. **メモリ効率**: 前方ワイヤが多い回路（典型的な組み合わせ回路）で状態サイズが大幅に縮小
3. **tick 完了時のクローン削減**: 全セル状態のクローンが不要。遅延ワイヤ分のみ更新

## 設計・方針

### 現行モデルの分析

```
┌─ Simulator ─────────────────────────────────┐
│  circuit: Circuit                            │
│  prev_state: SimState (HashMap<Pos, bool>)   │  ← 全セル分
│  curr_state: SimState (HashMap<Pos, bool>)   │  ← 全セル分
│  tick: u64                                   │
│  cell_index: usize                           │
└──────────────────────────────────────────────┘
```

**tick 処理フロー:**
1. `cell_index = 0` から全セルを辞書順に処理
2. 各セルの入力ワイヤを走査し、前方なら `curr_state`、後方なら `prev_state` から値を読む
3. OR 合成して `curr_state[cell]` に書き込む
4. 全セル処理後、`prev_state = curr_state.clone()`（**全セルのクローン**）

**問題点:**
- `prev_state` は遅延ワイヤの参照時のみ使用されるが、全セルの値を保持している
- tick 完了時に全セルのクローンが毎回発生する

### ワイヤ状態モデル

状態の保持単位を「遅延ワイヤのソースセル値」に変更する。

```
┌─ WireSimulator ─────────────────────────────┐
│  circuit: Circuit                            │
│  delayed_values: Vec<bool>                   │  ← 遅延ワイヤ分のみ
│  cell_values: Vec<bool>                      │  ← tick 中の計算用
│  tick: u64                                   │
│  cell_index: usize                           │
│  delayed_wire_indices: Vec<usize>            │  ← 事前計算
│  stateless_cells: Vec<usize>                 │  ← 入力なしセルの索引
└──────────────────────────────────────────────┘
```

#### 状態の構成要素

| 要素 | 格納先 | 説明 |
|------|--------|------|
| 遅延ワイヤ値 | `delayed_values` | 前 tick 終了時の遅延ワイヤのソースセル値。tick 間で持続 |
| 計算中セル値 | `cell_values` | 現在 tick の処理中に計算されたセル値。インデックスで高速アクセス |
| 入力なしセル値 | `cell_values` 内 | 入力ワイヤを持たないセルは `delayed_values` から前 tick の値を復元 |

#### 遅延ワイヤの特定（構築時・事前計算）

```rust
/// ワイヤ i が遅延ワイヤかどうか
fn is_delayed(wire: &Wire) -> bool {
    wire.dst < wire.src  // 辞書順で後方
}
```

構築時に遅延ワイヤのインデックスを `delayed_wire_indices` としてリスト化する。

#### tick 処理フロー（新モデル）

1. Generator 等の入力を `cell_values` に適用
2. 各セルを辞書順に処理:
   - 入力ワイヤなし: `delayed_values` から前 tick の値を復元し `cell_values` に書き込む
   - 前方ワイヤのみ: `cell_values[src_index]` を直接参照（即時伝搬）
   - 後方ワイヤあり: `delayed_values[delayed_idx]` から前 tick の値を参照
3. tick 完了:
   - 各遅延ワイヤについて `delayed_values[i] = cell_values[src_cell_index]` を更新
   - 入力なしセルについて `delayed_values` の対応箇所を更新
   - **全セルクローン不要**

### 入力なしセルの扱い

入力ワイヤを持たないセル（Generator 対象でもない）は、現行モデルでは前 tick の値を保持する。ワイヤ状態モデルでは、これを「暗黙の自己遅延」として扱う:

- 構築時に入力なしセルを `stateless_cells` として列挙
- 各セルに対応する `delayed_values` のスロットを割り当てる
- tick 処理時に `delayed_values` から値を復元
- tick 完了時に `cell_values` の値を `delayed_values` に書き戻す

これにより、遅延ワイヤと入力なしセルが統一的に扱われる。

### インデックスベースの高速アクセス

現行モデルの `HashMap<Pos, bool>` を `Vec<bool>` に変更し、セルの辞書順インデックスで直接アクセスする:

```rust
/// セル位置 → sorted_cells 内のインデックス
cell_pos_to_index: HashMap<Pos, usize>  // 構築時に計算
```

`cell_values[index]` での O(1) アクセスにより、HashMap のオーバーヘッドを排除する。

### 出力形式— AllCell / ViewPort

tick 完了時に保持するセル状態の出力形式を 2 種類提供する:

- **AllCell**: すべてのセルの状態を `Map<Pos, bool>` で保持する。バッチ実行・JSON 出力・テスト向け
- **ViewPort**: 指定された `Vec<Rect>` の範囲内のセルのみ `Map<Pos, bool>` で保持する。Web UI・ビューモード向け

詳細は [output-format.md](output-format.md) を参照。

### 旧実装との関係

旧 `Simulator` / `SimState` は削除し、`WireSimulator` / `WireSimState` で完全に置き換える。

詳細は [trait-design.md](trait-design.md) を参照。

### 公開 API の互換性

`WasmSimulator`, `io::json`, CLI 等の利用側は `WireSimulator` を直接使用するよう更新する。`SimState` 型は廃止し、セル値の参照には `get_cell()` / `cell_values()` を用いる。

## ステップ

1. **`WireSimState` の実装**: 遅延ワイヤベースの状態管理（[wire-sim-state.md](wire-sim-state.md)）
2. **`WireSimulator` の実装**: ワイヤ状態モデルのシミュレーションエンジン（[wire-simulator.md](wire-simulator.md)）
3. **出力形式の実装**: AllCell / ViewPort 形式の対応（[output-format.md](output-format.md)）
4. **利用側の移行**: `WasmSimulator`, `io::json`, CLI を `WireSimulator` へ直接切り替え（[trait-design.md](trait-design.md)）
5. **旧実装の削除**: `Simulator`, `SimState`, `StateMut` を削除
6. **テスト**: テストマニフェスト・エンジンテストで `WireSimulator` を検証
