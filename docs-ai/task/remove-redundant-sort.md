# simulate_to_output_json 内の冗長なソートを除去

`simulate_to_output_json` が毎 tick ごとに冗長なソートを行っている問題を解決する。

作成日: 2026-03-23
ステータス: 設計完了（未実装）

## 背景・動機

`io/json.rs` の `simulate_to_output_json` 内で、毎 tick ごとに `simulator.state().values().keys()` を取得してソートしているが、`Circuit` は内部に `sorted_cells` を保持済み。`Simulator` から `Circuit` の `sorted_cells()` にアクセスする公開 API がないため、現状は冗長なソートを行っている。

重要度: low

## 設計・方針

`Simulator` に `circuit()` アクセサを追加し、`circuit.sorted_cells()` を利用する。

- 影響範囲: `simulation/engine.rs`（アクセサ追加）, `io/json.rs`（ソートの置き換え）

## 代替案の検討: state をソート済みにする

### 概要

`sorted_cells` を `Circuit` に持たせる代わりに、`SimState` 自体をソート済みにして順序を保証する案を検討した。

### 案A: SimState を `BTreeMap<Pos, bool>` に変更

`BTreeMap` はキー順にソートされるため、イテレーション時にソート済みの出力が得られる。

- **長所**: `sorted_cells` が不要になり、ソート順が型で保証される
- **短所**:
  - `get` / `set` が O(1) → O(log n) に劣化。`Simulator::step()` 内で全セル・全ワイヤのアクセスに毎回呼ばれるため、ホットパスの性能に直接影響する
  - インデックスによるランダムアクセス（`cell_index`）が不可。エンジンの `step()` は `sorted_cells[cell_index]` で O(1) アクセスしており、BTreeMap ではこれを再現できない

**評価: 不採用** — ホットパスの性能劣化とインデックスアクセス不可が致命的。

### 案B: SimState を `Vec<(Pos, bool)>`（Pos 順ソート済み）に変更

ソート済み Vec により、インデックスアクセス O(1) と順序保証を両立する。

- **長所**: `state.values[cell_index]` で O(1) アクセス可能。`sorted_cells` を削除できる
- **短所**:
  - `get(pos)` / `set(pos, value)` が O(1) → O(log n)（二分探索が必要）
  - `step()` で `self.prev_state.get(wire.src)` や `self.curr_state.get(wire.src)` を任意の Pos でルックアップしており、これがセル数 × ワイヤ数に比例して呼ばれる

**評価: 不採用** — ランダムアクセスの性能劣化がホットパスのボトルネックになる。

### 案C: SimState を `Vec<bool>` + `Circuit` に `pos_to_index: HashMap<Pos, usize>` を追加

state をインデックスベースの `Vec<bool>` にし、Pos ↔ index の変換を `Circuit` 側に持たせる。

- **長所**: `get(index)` / `set(index, value)` が O(1)。キャッシュフレンドリー
- **短所**:
  - `sorted_cells` は削除できない（index → Pos の逆引きに必要）
  - `pos_to_index` という新しいデータ構造が増え、むしろ複雑化する
  - `SimState` の API が `Pos` ベースから `usize` ベースに変わり、`ViewRenderer` 等の外部利用箇所すべてに影響

**評価: 不採用** — `sorted_cells` を削除する目的を達成できず、かつ複雑性が増す。

### 総合評価

`sorted_cells` は `BTreeSet<Pos>` を `Vec<Pos>` にキャッシュしたものであり、冗長に見えるが以下の理由で妥当:

1. **処理順序は回路トポロジの性質** — SimState（値の入れ物）ではなく Circuit（構造定義）が持つべき責務
2. **ホットパスの性能** — HashMap の O(1) ルックアップは `step()` のセル数 × ワイヤ数回呼ばれるため、O(log n) への劣化は許容しにくい
3. **BTreeSet と Vec の二重保持のコスト** — メモリ上のオーバーヘッドはセル数に比例するのみで、実用上無視できる

**結論: `sorted_cells` は現行のまま `Circuit` に保持し、当タスクでは `Simulator` に `circuit()` アクセサを追加する方針で進める。**

## 関連タスク

- [cell-merge-optimization.md](cell-merge-optimization.md): セルマージ最適化でも `Simulator::circuit()` アクセサが有用。両タスクは独立して実装可能。
