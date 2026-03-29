# LGCELL2 アーキテクチャ概要

LGCELL2 は、論理回路をグリッドベースの有向グラフとしてモデリングし、ステップ単位でシミュレーションを実行するシステムである。

## 基本コンセプト

- **セル（Cell）**: グリッド座標 `(x, y)` 上に配置される計算単位。`bool` 値（0 / 1）を保持する。
- **ワイヤ（Wire）**: セル間を結ぶ有向辺。Positive（そのまま伝搬）と Negative（反転伝搬）の 2 種類の極性を持つ。
- **回路（Circuit）**: セルとワイヤの集合で構成される有向グラフ。構築後は不変。

## OR ロジックによる万能性

複数のワイヤが 1 つのセルに接続される場合、伝搬値の **OR（最大値）** を取る。この単純なルールと Negative ワイヤの組合せにより、NAND ゲートが実現できる。NAND は万能ゲートであるため、あらゆる論理関数を構成可能。

```
NAND(a, b) = a →(Neg)→ out, b →(Neg)→ out
```

入力が両方 1 のとき: NOT(1) OR NOT(1) = 0 OR 0 = 0
それ以外のとき: 少なくとも一方が NOT(0) = 1 → OR = 1

## モジュール構成

```
lib.rs
├── circuit/    回路データモデル（Pos, Wire, Circuit）
├── simulation/ シミュレーションエンジン（WireSimState, WireSimulator）
├── io/         JSON 入出力
├── view/       TUI レンダラ
└── wasm_api/   WASM エクスポート
```

依存方向: `io` / `view` / `wasm_api` → `simulation` → `circuit`（上位が下位に依存）

## 関連ドキュメント

- [data-model.md](data-model.md) — セル・ワイヤ・回路のデータモデル詳細
- [simulation-model.md](simulation-model.md) — 伝搬ルールとシミュレーションエンジンの仕組み
- [circuit-examples.md](circuit-examples.md) — 基本ゲートから半加算器までの回路構成例
