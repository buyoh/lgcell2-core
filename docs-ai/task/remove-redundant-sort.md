# simulate_to_output_json 内の冗長なソートを除去

`simulate_to_output_json` が毎 tick ごとに冗長なソートを行っている問題を解決する。

作成日: 2026-03-23
ステータス: 未着手

## 背景・動機

`io/json.rs` の `simulate_to_output_json` 内で、毎 tick ごとに `simulator.state().values().keys()` を取得してソートしているが、`Circuit` は内部に `sorted_cells` を保持済み。`Simulator` から `Circuit` の `sorted_cells()` にアクセスする公開 API がないため、現状は冗長なソートを行っている。

重要度: low

## 設計・方針

`Simulator` に `circuit()` アクセサを追加し、`circuit.sorted_cells()` を利用する。

- 影響範囲: `simulation/engine.rs`（アクセサ追加）, `io/json.rs`（ソートの置き換え）
