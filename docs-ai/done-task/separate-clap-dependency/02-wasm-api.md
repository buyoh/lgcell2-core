# フェーズ 2: WASM API レイヤー実装

作成日: 2026-03-24
ステータス: 完了

## 概要

`wasm-bindgen` を使用して、回路シミュレーション機能を JavaScript から呼び出せる API としてエクスポートする。

## 設計

### エクスポートする関数

既存の `io::json` モジュールの機能をラップし、JSON 文字列ベースの API を提供する。

```rust
// src/wasm_api.rs

use wasm_bindgen::prelude::*;
use crate::io::json::{parse_circuit_json, simulate_to_output_json, output_json_to_string};

/// 回路 JSON を受け取り、シミュレーション結果 JSON を返す。
///
/// # Arguments
/// * `circuit_json` - 回路定義 JSON 文字列
/// * `ticks` - シミュレーションする tick 数
///
/// # Returns
/// シミュレーション結果 JSON 文字列。エラー時は JavaScript 例外をスローする。
#[wasm_bindgen]
pub fn simulate(circuit_json: &str, ticks: u64) -> Result<String, JsError> {
    let circuit = parse_circuit_json(circuit_json)
        .map_err(|e| JsError::new(&e))?;
    let output = simulate_to_output_json(circuit, ticks);
    output_json_to_string(&output)
        .map_err(|e| JsError::new(&e))
}
```

### API 設計方針

- **最初はシンプルな関数 API** から始める。nospace20 のようなステートフルな VM ラッパーは、lgcell2-core ではシミュレーションが tick ベースの一括実行のため不要
- **JSON 文字列の入出力** を基本とする。`serde-wasm-bindgen` による `JsValue` 直接変換は、将来的にパフォーマンスが必要になった場合に検討する
- **エラーは `JsError`** で JavaScript 側に伝搬する

### 将来の拡張候補（このフェーズでは実装しない）

- `parse_circuit(circuit_json: &str) -> Result<JsValue, JsError>`: 回路の構文チェックのみ
- `serde-wasm-bindgen` を使った `JsValue` 直接返却（JSON 文字列のパースを JS 側で省略）
- ステートフルなシミュレータラッパー（tick 単位の実行制御が必要になった場合）

## ステップ

1. `src/wasm_api.rs` を作成（上記コード）
2. `src/lib.rs` の `#[cfg(feature = "wasm")] pub mod wasm_api;` を有効化
3. `cargo build --target wasm32-unknown-unknown --lib --no-default-features --features wasm` でビルドが通ることを確認

## 実施内容

- `src/wasm_api.rs` を追加し、`simulate(circuit_json: &str, ticks: u64) -> Result<String, JsError>` を実装
- `src/lib.rs` 側で `wasm` feature 時の公開を有効化
- `src/wasm_api.rs` にユニットテストを追加
    - 正常系: 3 tick 実行で結果 JSON に `ticks` が含まれる
    - 異常系: 非 wasm 環境で `JsError` 生成が panic する制約があるため `wasm32` 限定テストとして定義

## 検証結果

- `cargo build --target wasm32-unknown-unknown --lib --no-default-features --features wasm`: 成功
- `cargo test --no-default-features --features wasm --lib`: 成功
