# フェーズ 3: WASM ビルドスクリプト整備

作成日: 2026-03-24
ステータス: 未着手

## 概要

wasm-pack を使った WASM ビルドスクリプトと、サイズ最適化のプロファイル設定を整備する。

## 設計

### ビルドスクリプト `build-wasm.sh`

nospace20 の `build-wasm.sh` を参考に、プロジェクトルートに配置する。

```bash
#!/bin/bash

set -eu
cd "$(dirname "$0")"

# release
cargo build --release \
  --target wasm32-unknown-unknown --lib \
  --no-default-features --features wasm
wasm-pack build --target bundler --no-default-features --features wasm

# debug (オプション)
if [ "${NO_DEBUG:-false}" != "true" ]; then
  cargo build \
    --target wasm32-unknown-unknown --lib \
    --no-default-features --features wasm
  wasm-pack build --dev --out-dir pkg-dev --target bundler --no-default-features --features wasm
fi
```

### 出力先

- `pkg/`: release ビルドの出力（wasm-pack のデフォルト）
- `pkg-dev/`: debug ビルドの出力

両ディレクトリは `.gitignore` に追加する。

### Cargo.toml への追記

```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--enable-bulk-memory"]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

注意: `[profile.release]` の設定はネイティブビルドにも影響する。ネイティブ側のデバッグ・パフォーマンスへの影響が許容できない場合は、wasm 専用のプロファイルを検討する。

### 前提条件

以下のツールが必要:

- `wasm-pack`: `cargo install wasm-pack`
- `wasm32-unknown-unknown` ターゲット: `rustup target add wasm32-unknown-unknown`

## ステップ

1. `build-wasm.sh` を作成し実行権限を付与
2. `.gitignore` に `pkg/` と `pkg-dev/` を追加
3. `Cargo.toml` に wasm-pack profile と release profile を追記
4. `./build-wasm.sh` を実行してビルドが成功することを確認
5. `pkg/` ディレクトリに `.wasm`, `.js`, `.d.ts` ファイルが生成されることを確認

## 備考

- `--target bundler` は ES Modules 形式で出力する。Node.js から直接使用する場合は、WASM ファイルの手動読み込みが必要（フェーズ 4 参照）
- `--target nodejs` にすると Node.js 専用の出力になるが、ブラウザ互換性を考慮して `bundler` を採用する
