# シミュレーション最適化: セルのマージ（Union-Find）

即時伝搬の Positive ワイヤで接続された等価なセルを Union-Find で集約し、シミュレーション時の冗長な計算を省略する。

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

現在の `Simulator` は、毎 tick ごとに `sorted_cells` の全セルを 1 つずつ処理する。しかし、即時伝搬の Positive ワイヤのみで接続されたセル群は、常に同一の値を持つ。これらのセルを 1 つの代表セルにまとめることで、冗長な計算を省略できる。

例: `A → B → C`（全て即時 Positive ワイヤ、B と C に他の入力ワイヤなし）の場合、B と C は常に A と同じ値を持つ。A の計算結果を B, C にコピーするだけで済む。

## 設計・方針

### マージ可能条件

セル B がセル A（ソースの代表元グループ）に **マージ可能** となる条件:

1. B の全入力ワイヤが **Positive** であること（反転なし）
2. B の全入力ワイヤが **即時伝搬** であること（`dst >= src` 座標順）
3. B の全入力ワイヤの src が **同一の Union-Find グループ** に属すること

条件 1〜3 を満たす場合、B の値は OR(A, A, ..., A) = A となり、代表元の値と常に一致する。独立した計算が不要となる。

### Union-Find アルゴリズムによる集約手順

1. 全セルを個別の集合として初期化
2. `sorted_cells` の順序（座標昇順）でセルを走査
3. 各セルについてマージ可能条件を判定
4. 条件を満たすセルをソースの代表元グループに union
5. 最終的に、各 Union-Find グループの代表元のみがシミュレーション時に実際の計算対象となる

座標順に走査するため、ソース側の Union-Find グループは走査時点で確定済みであり、正しく判定できる。

### モジュール構成

単一責任原則に従い、以下のように分割する:

```
src/
  optimization/
    mod.rs              // モジュール公開
    union_find.rs       // Union-Find データ構造（汎用）
    cell_merger.rs      // セルマージロジック（Circuit → CellMergeMap）
```

#### `union_find.rs`

汎用的な Union-Find（素集合データ構造）の実装。path compression と union by rank を適用する。
Circuit 固有のロジックは含めない。

```rust
pub struct UnionFind<T: Eq + Hash + Copy> { ... }

impl<T: Eq + Hash + Copy> UnionFind<T> {
    pub fn new() -> Self
    pub fn make_set(&mut self, item: T)
    pub fn find(&mut self, item: T) -> T        // 代表元を返す
    pub fn union(&mut self, a: T, b: T)         // 2つの集合を統合
    pub fn same_set(&mut self, a: T, b: T) -> bool
}
```

#### `cell_merger.rs`

`Circuit` を受け取り、マージ可能なセルを Union-Find で集約する。
Union-Find の利用とマージ条件の判定のみを責務とする。

```rust
/// セルのマージ結果。Union-Find によるセルの等価グループ情報を保持する。
pub struct CellMergeMap {
    /// 各セルの代表元
    representative: HashMap<Pos, Pos>,
    /// 代表元セルのみのソート済みリスト
    representatives: Vec<Pos>,
    /// 各代表元に属する従属セルのリスト
    dependents: HashMap<Pos, Vec<Pos>>,
}

impl CellMergeMap {
    /// Circuit を解析してマージマップを構築する。
    pub fn from_circuit(circuit: &Circuit) -> Self

    /// 指定セルの代表元を返す。
    pub fn representative(&self, pos: Pos) -> Pos

    /// 代表元セルの一覧を返す。
    pub fn representatives(&self) -> &[Pos]

    /// 指定セルが代表元かどうかを返す。
    pub fn is_representative(&self, pos: Pos) -> bool

    /// 代表元に属する従属セルの一覧を返す。
    pub fn dependents(&self, representative: Pos) -> &[Pos]
}
```

### Simulator への統合

`Simulator` に `CellMergeMap` フィールドを追加し、構築時に自動で計算する:

```rust
pub struct Simulator {
    circuit: Circuit,
    merge_map: CellMergeMap,  // 追加
    prev_state: SimState,
    curr_state: SimState,
    tick: u64,
    cell_index: usize,
}
```

`step()` メソッドの変更:

```
step():
  cell = sorted_cells[cell_index]

  if merge_map.is_representative(cell):
    // 通常の計算ロジック（既存と同じ）
  else:
    representative = merge_map.representative(cell)
    curr_state[cell] = curr_state[representative]  // 代表元から値をコピー
```

代表元は座標順で従属セルより前に処理される（即時伝搬条件 `dst >= src` による）。そのため、従属セルの処理時点で代表元の値は `curr_state` に確定済みである。

### マージ不可能な場合

マージ可能なセルが存在しない場合、`CellMergeMap` は全セルが代表元となる。すなわち `is_representative()` は常に true を返し、`step()` は既存ロジックのみ実行する。追加のオーバーヘッドは `is_representative()` のハッシュマップ参照のみ。

### 外部 API への影響

- `step()` の粒度（セル 1 個単位）は変わらない
- `state()`, `state_mut()` は全セルの値を返す（従属セル含む）
- `run_with_snapshots()` は全セルの値を含むスナップショットを返す
- 既存の全てのテストが変更なしで通ることを確認する

## ステップ

1. `optimization/union_find.rs` — Union-Find データ構造の実装とユニットテスト
2. `optimization/cell_merger.rs` — `CellMergeMap` の実装とユニットテスト
3. `simulation/engine.rs` — `Simulator` に `CellMergeMap` を統合
4. 既存テストの動作確認・結合テスト追加
5. アーキテクチャドキュメント更新

## 関連タスク

- [remove-redundant-sort.md](remove-redundant-sort.md): `Simulator::circuit()` アクセサの追加を提案。本タスクでも Simulator が元の `Circuit` を保持する必要があり、同アクセサが有用。両タスクは独立して実装可能。
