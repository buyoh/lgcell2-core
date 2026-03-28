# ワイヤ状態モデルによるシミュレーション再設計

シミュレーションの内部状態モデルを「セル状態」から「ワイヤ状態」へ移行し、旧実装との共通トレイトを定義する。

作成日: 2026-03-28
ステータス: 設計中

## 背景・動機

詳細は [design.md](./design.md) を参照。

### 問題意識

現在の `Simulator` はセルごとに `prev_state`・`curr_state` を保持する（`SimState = HashMap<Pos, bool>`）。
しかし現仕様では **辞書順後方のセルへは 0 tick で伝搬**する。これらのセル値は各 tick 内で前のセルの計算後に即座に求まるため、「セルが状態を持つ」という設計は旧仕様的であり冗長である。

正しい概念モデルは：

> **遅延を生じさせるのは、辞書順前方（dst < src）を宛先とするワイヤだけである。**  
> このような「後退ワイヤ」のみが tick 間の状態を保持する。前進ワイヤは状態を持たない。

これに基づきワイヤ状態モデル (`WireState`) での新しいシミュレータ `WireSimulator` を設計・実装する。
旧実装 (`Simulator`) は互換性のために保持し、共通トレイト `Simulate` を定義して両実装を統一する。

## ドキュメント構成

| ファイル | 内容 |
|---|---|
| [README.md](./README.md) | このファイル。概要・進捗管理 |
| [design.md](./design.md) | データ構造・アルゴリズム・トレイト設計の詳細 |

## 実装ステップ

1. [ ] `Simulate` トレイトの定義 (`src/simulation/simulate.rs`)
2. [ ] `WireState` の定義 (`src/simulation/wire_state.rs`)
3. [ ] `WireSimulator` の実装 (`src/simulation/wire_engine.rs`)
4. [ ] 旧 `Simulator` に `Simulate` トレイトを実装
5. [ ] テストの追加 (`wire_state_tests.rs`, `wire_engine_tests.rs`)
6. [ ] `mod.rs` の公開 API 更新
7. [ ] ドキュメント (`AGENTS.md`, `simulation-model.md`) 更新
