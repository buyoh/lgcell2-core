# データモデル

回路を構成する 3 つの基本要素（Pos, Wire, Circuit）の設計と、その関係を解説する。

## Pos — グリッド座標

```rust
pub struct Pos {
    pub x: i32,
    pub y: i32,
}
```

セルの位置を表すグリッド座標。`x` を先に宣言することで、derive `Ord` による比較順が **(x, y) の辞書順** となる。この順序がシミュレーションにおけるセルの処理順序を直接決定する（詳細は [simulation-model.md](simulation-model.md) を参照）。

## WireKind — ワイヤの極性

```rust
pub enum WireKind {
    Positive,  // そのまま伝搬: v → v
    Negative,  // 反転して伝搬: v → !v
}
```

2 種類の極性が、最小限の機構で万能ゲートの構成を可能にする。

## Wire — 有向辺

```rust
pub struct Wire {
    pub src: Pos,
    pub dst: Pos,
    pub kind: WireKind,
}
```

`src` から `dst` へ信号を伝搬する有向辺。`propagate(src_value)` メソッドで極性に応じた値変換を行う。

**制約:**
- **self-loop 禁止**: `src == dst` のワイヤは構築時にエラーとなる。同一 tick 内での振動を原理的に排除するための設計。
- **有向ペアの一意性**: 同一の `(src, dst)` ペアに対して複数のワイヤは配置できない。`WireKind` に関わらず、ある 2 点間には最大 1 本のワイヤのみ許可される。

## Circuit — 回路定義

```rust
pub struct Circuit {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    incoming: HashMap<Pos, Vec<usize>>,
    sorted_cells: Vec<Pos>,
}
```

構築後は **不変** であり、シミュレーション中に構造が変わることはない。

### フィールドの役割

| フィールド | 型 | 目的 |
|---|---|---|
| `cells` | `BTreeSet<Pos>` | 全セル座標。`BTreeSet` により常に `(x, y)` 辞書順にソート済み |
| `wires` | `Vec<Wire>` | 全ワイヤの定義 |
| `incoming` | `HashMap<Pos, Vec<usize>>` | dst → ワイヤインデックスの逆引き。シミュレーション時の O(1) ルックアップ用 |
| `sorted_cells` | `Vec<Pos>` | `cells` をベクタ化したもの。ステップ実行時のインデックスアクセス用 |

### 構築時の検証

`Circuit::new()` は以下を検証し、不正な回路を拒否する:

1. **self-loop がないこと** — `wire.src == wire.dst` を禁止
2. **ワイヤの端点がセル集合に含まれること** — `wire.src` と `wire.dst` の両方が `cells` に存在すること
3. **ワイヤペアの一意性** — 同一の `(src, dst)` を持つワイヤは 1 つのみ許可。重複検出時にエラーを返す

### 事前計算による最適化

`incoming` マップと `sorted_cells` は構築時に一度だけ計算され、シミュレーション中は参照のみ。これにより、セル処理ごとの入力ワイヤ探索がハッシュテーブルの O(1) ルックアップで済む。

## セル値の合成ルール

1 つのセルに複数の入力ワイヤが接続される場合:

```
cell_value = max(propagated values from all incoming wires)
```

- **入力ワイヤなし**: 前 tick の値を保持（入力セルとして外部から値を設定する想定）
- **Positive ワイヤ**: `src_value` をそのまま伝搬
- **Negative ワイヤ**: `!src_value` を伝搬
- **複数ワイヤ**: 全伝搬値の最大値を取る（= OR 演算）

`bool` における `max(false, true) = true` であるため、OR と等価になる。

## JSON との対応

JSON 入力では `cells` フィールドは存在せず、ワイヤの端点 (`src`, `dst`) から自動推論される。JSON スキーマと内部モデルは `TryFrom<CircuitJson> for Circuit` で変換され、スキーマ変更の影響が内部に波及しないよう隔離されている。

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
  ]
}
```

この例では、セル `(0,0)` と `(1,0)` がワイヤ端点から自動的に推論・登録される。

## サブ回路データモデル

サブ回路導入後は、`Circuit` がモジュールインスタンス列を保持する。

```rust
pub struct Circuit {
  cells: BTreeSet<Pos>,
  wires: Vec<Wire>,
  incoming: HashMap<Pos, Vec<usize>>,
  sorted_cells: Vec<Pos>,
  modules: Vec<ResolvedModule>,
}
```

`ResolvedModule` は、モジュールインスタンスに必要な情報をすべて解決済みで保持する値オブジェクトである。

```rust
pub struct ResolvedModule {
  sub_circuit: Circuit,
  input: Vec<Pos>,
  output: Vec<Pos>,
  sub_input: Vec<Pos>,
  sub_output: Vec<Pos>,
}
```

### ResolvedModule の役割

- `sub_circuit`: 子回路本体
- `input`: 親回路上の入力ポート
- `output`: 親回路上の出力ポート
- `sub_input`: 子回路側の入力ポート
- `sub_output`: 子回路側の出力ポート

この 5 要素を 1 つに束ねることで、シミュレーション時に親子間の値転送（入力注入と出力反映）をインデックスベースで実行できる。

### 構築時検証（モジュール関連）

`Circuit::with_modules()` は既存のワイヤ検証に加え、以下を検証する。

1. モジュール出力セルへの入力ワイヤ禁止
2. モジュール出力セルの重複禁止
3. ポート列制約（同一 x、連続 y）
4. 出力列が入力列より後方（x が大きい）
