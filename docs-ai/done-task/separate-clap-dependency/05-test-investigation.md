# 追加テスト失敗の調査メモ

作成日: 2026-03-24
ステータス: 解決済み

## 事象 1: Node.js テストで `TypeError: Cannot convert 3 to a BigInt`

- 発生箇所: `tools/wasm-test/test.mjs`
- 原因: WASM エクスポート関数 `simulate` の第2引数は Rust 側で `u64` であり、JS では `BigInt` が必要。
- 対応: 呼び出し値を `3`, `1` から `3n`, `1n` に変更。

## 事象 2: `cargo test --no-default-features --features wasm --lib` で panic

- 発生箇所: `src/wasm_api.rs` の異常系テスト
- 原因: `wasm_bindgen::JsError::new` は非 wasm ターゲットで使用できず panic する。
- 対応: 異常系テストを `#[cfg(target_arch = "wasm32")]` で wasm32 限定に変更。

## 再確認

- `cargo test --no-default-features --features wasm --lib`: 成功
- `node tools/wasm-test/test.mjs`: 成功
