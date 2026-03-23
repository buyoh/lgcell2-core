# 初期実装: 回路データモデル・シミュレーション・CLI

空の Cargo プロジェクトに対し、LGCELL2 の基本構成要素（Cell, Wire）、ステップ実行可能なシミュレーションエンジン、JSON 入出力 & CLI を実装する。

作成日: 2026-03-23
ステータス: 設計完了（未実装）

## 背景・動機

lgcell2-core は論理回路をグリッドベースで表現・シミュレーションする Rust クレートである。将来的に wasm 化して lgcell2-webui から利用する。現時点ではプロジェクトが空のため、最初の一歩として以下を揃える必要がある。

- 回路のデータモデル (Cell, Wire, Circuit)
- 中断可能なステップ実行シミュレーションエンジン
- JSON 形式での回路読み込みと結果出力
- CLI エントリポイント

## サブタスク一覧

| # | ドキュメント | 概要 |
|---|---|---|
| 1 | [data-model.md](data-model.md) | Cell, Wire, Circuit のデータ構造設計 |
| 2 | [simulation.md](simulation.md) | 伝搬ルール・ステップ実行エンジンの設計 |
| 3 | [io-cli.md](io-cli.md) | JSON フォーマット・CLI 設計 |

## ステップ

1. データモデル (`circuit` モジュール) を実装しユニットテストを書く
2. シミュレーションエンジン (`simulation` モジュール) を実装しユニットテストを書く
3. JSON I/O (`io` モジュール) を実装しユニットテストを書く
4. CLI エントリポイントを実装する
5. 統合テスト (半加算器等の小規模回路) を書く

## ディレクトリ構成（実装後の想定）

```
src/
  lib.rs               # クレートルート (pub mod 宣言)
  bin/
    lgcell2/
      main.rs          # CLI エントリポイント (clap)
  circuit/
    mod.rs
    cell.rs            # Pos
    cell_tests.rs      # Pos の単体テスト
    wire.rs            # WireKind, Wire
    wire_tests.rs      # Wire の単体テスト
    circuit.rs         # Circuit (回路定義, 不変)
    circuit_tests.rs   # Circuit の単体テスト
  simulation/
    mod.rs
    state.rs           # SimState (各セルの現在値)
    state_tests.rs     # SimState の単体テスト
    engine.rs          # Simulator, StepResult
    engine_tests.rs    # Simulator の単体テスト
  io/
    mod.rs
    json.rs            # JSON デシリアライズ / シリアライズ
    json_tests.rs      # JSON I/O の単体テスト
tests/
  half_adder.rs        # 半加算器等の統合テスト
```

### テスト規約

- **単体テスト**: 各モジュールと同階層に `foo_tests.rs` として配置。`#[cfg(test)] mod tests` は使わない。
  - `foo.rs` 内に `#[cfg(test)] mod foo_tests;` で参照するか、`mod.rs` で `#[cfg(test)] mod foo_tests;` を宣言する。
- **統合テスト**: `tests/` ディレクトリに配置。回路単位のエンドツーエンドテスト。
