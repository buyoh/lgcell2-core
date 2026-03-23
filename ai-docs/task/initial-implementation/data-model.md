# データモデル設計

Cell, Wire, Circuit の構造体定義とその関係を設計する。

作成日: 2026-03-23
ステータス: 設計完了（未実装）

## 背景・動機

LGCELL2 の回路は「セル（ノード）とワイヤ（辺）による有向グラフ」である。このグラフを効率的に表現し、シミュレーションエンジンや JSON I/O から利用できるデータモデルが必要。

## 設計・方針

### Pos — グリッド座標

```rust
/// グリッド上の座標。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}
```

- `Ord` の導出順は **(x, y) の辞書順** となる。これはシミュレーションの伝搬順序の定義と一致する。
- Rust の derive `Ord` はフィールド宣言順で比較するため、`x` を先に宣言する。

### CellValue — セルの値

```rust
/// セルが保持する値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellValue {
    /// 通常モード: 0 または 1。
    Binary(bool),
    /// 微分可能モード: 0.0〜1.0 の実数。
    Continuous(f64),
}
```

- 通常モードでは `Binary(false)` = 0, `Binary(true)` = 1。
- `Continuous` は将来の微分可能モード用。初期実装では `Binary` のみ使用する。

### WireKind — ワイヤの極性

```rust
/// ワイヤの極性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireKind {
    /// そのまま伝搬 (v)。
    Positive,
    /// 反転して伝搬 (1 - v)。
    Negative,
}
```

### Wire — 有向辺

```rust
/// セル間の信号伝搬を担う有向辺。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    pub src: Pos,
    pub dst: Pos,
    pub kind: WireKind,
}
```

### Circuit — 回路定義（不変）

```rust
/// 回路の構造定義。構築後は不変。
pub struct Circuit {
    /// 全セルの初期値。BTreeMap により (x, y) 順でソート済み。
    cells: BTreeMap<Pos, CellValue>,
    /// 全ワイヤ。
    wires: Vec<Wire>,
    /// dst でグループ化したワイヤインデックス（事前計算）。
    incoming: HashMap<Pos, Vec<usize>>,
    /// ソート済みセル座標リスト（事前計算）。
    sorted_cells: Vec<Pos>,
}
```

- `cells` は `BTreeMap` を使用し、キー順 = 処理順 = (x, y) 辞書順を保証する。
- `incoming` はシミュレーション時の高速ルックアップ用。`Circuit` 構築時に `wires` から計算する。
- `sorted_cells` は `cells` のキーをベクタ化したもの。ステップ実行時にインデックスアクセスするために保持する。

### セル値の合成ルール

1 つのセルに複数のワイヤが接続される場合の合成ルールを定義する。

**OR (max) を採用する:**

```
cell_value = max(propagated values from all incoming wires)
```

- 入力ワイヤがない場合: 初期値を保持する（Input UI が接続される想定）。
- Positive ワイヤ: `v` をそのまま伝搬。
- Negative ワイヤ: `1 - v` を伝搬。
- 複数ワイヤ: 伝搬値の最大値 (OR) を取る。

**OR 採用の根拠:**

Negative ワイヤ 2 本で NAND が実現でき、NAND は万能ゲートであるため、すべての論理関数を構成可能。

| ゲート | 実現方法 |
|--------|----------|
| NOT(a) | a → (Negative) → out |
| NAND(a,b) | a → (Negative) → out, b → (Negative) → out |
| AND(a,b) | NAND → NOT (2セル) |
| OR(a,b) | a → (Positive) → out, b → (Positive) → out |
| NOR(a,b) | OR → NOT (2セル) |
| XOR(a,b) | NAND, AND, OR の組合せ (5セル) |

### テスト方針

- `Pos` の `Ord` が (x, y) 辞書順であることを確認
- `Circuit` 構築時に `incoming` が正しく計算されることを確認
- `CellValue` の各バリアントの基本操作を確認
