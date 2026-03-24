# フェーズ 1: feature flag による clap 依存分離

作成日: 2026-03-24
ステータス: 完了

## 概要

`clap` を optional dependency にし、`cli` feature flag で管理する。wasm ビルド時には `--no-default-features --features wasm` でビルドすることで clap を除外できるようにする。

## 設計

### Cargo.toml の変更

現在:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

変更後:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["cli"]
cli = ["dep:clap"]
wasm = ["dep:wasm-bindgen", "dep:serde-wasm-bindgen"]

[[bin]]
name = "lgcell2"
path = "src/bin/lgcell2/main.rs"
required-features = ["cli"]

[dependencies]
clap = { version = "4", features = ["derive"], optional = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
```

ポイント:

- `crate-type = ["cdylib", "rlib"]`: `cdylib` は WASM ビルドに必要、`rlib` はネイティブライブラリ・テスト用
- `default = ["cli"]`: `cargo build` / `cargo test` はこれまで通り動作
- `required-features = ["cli"]`: wasm ビルド時にバイナリのコンパイルをスキップ
- `wasm` feature はこのフェーズでは定義のみ。依存の追加もこの時点で行うが、使用はフェーズ 2

### src/lib.rs の変更

WASM API モジュールの条件付きコンパイルを追加（フェーズ 2 で実際のコードを実装）:

```rust
pub mod circuit;
pub mod io;
pub mod simulation;

#[cfg(feature = "wasm")]
pub mod wasm_api;
```

## ステップ

1. `Cargo.toml` を上記の通り修正
2. `src/lib.rs` に `#[cfg(feature = "wasm")]` 行を追加（空モジュールで良い）
3. `cargo build` で既存のネイティブビルドが通ることを確認
4. `cargo test` で既存テストがすべてパスすることを確認
5. `cargo build --target wasm32-unknown-unknown --lib --no-default-features` で wasm ターゲットへのビルドが通ることを確認（wasm feature なしでもライブラリ部分は通るはず）

## 注意事項

- `build.rs` で `serde_yaml` を使用しているが、これは `[build-dependencies]` なのでランタイム依存には含まれない。wasm ビルドに影響しない
- `edition = "2024"` を使用している。wasm-pack との互換性を確認する必要がある

## 実施内容

- `Cargo.toml` に `[lib] crate-type = ["cdylib", "rlib"]` を追加
- feature flag を導入（`default = ["cli"]`, `cli`, `wasm`）
- `clap` を optional dependency 化し、`wasm-bindgen` / `serde-wasm-bindgen` を optional 追加
- `[[bin]]` に `required-features = ["cli"]` を追加
- `src/lib.rs` で `#[cfg(feature = "wasm")] pub mod wasm_api;` を追加

## 検証結果

- `cargo build`: 成功
- `cargo test`: 成功
- `cargo build --target wasm32-unknown-unknown --lib --no-default-features`: 成功
