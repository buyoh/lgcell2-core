# clap 依存をライブラリ crate から分離

`clap` が library crate 全体の依存に含まれており、wasm ビルドが失敗する問題を解決する。

作成日: 2026-03-23
ステータス: 未着手

## 背景・動機

`clap` は `src/bin/lgcell2/main.rs` でのみ使用されるが、`[dependencies]` に含まれているため library crate 全体の依存となっている。`lgcell2-core` は wasm ライブラリとしても使用される想定だが、`clap` は wasm32 ターゲットではコンパイルできず、wasm ビルドが失敗する。

重要度: high

## 設計・方針

### 案 A: バイナリを別 crate に分離 (推奨)

`lgcell2` バイナリを workspace member として分離し、`clap` はそちらの `Cargo.toml` に記載する。

- 影響範囲: `Cargo.toml`, ディレクトリ構造の変更
- 利点: ライブラリの依存が完全にクリーンになる
- 欠点: workspace 化の作業が必要

### 案 B: feature flag で分離

`clap` を optional dependency にし、binary に `required-features` を設定する。

- 影響範囲: `Cargo.toml` のみ
- 利点: 構造変更が最小限
- 欠点: feature flag の管理が必要
