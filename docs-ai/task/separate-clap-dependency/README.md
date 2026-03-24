# clap 依存分離と WASM ビルド対応

clap 依存をライブラリ crate から分離し、wasm ターゲットでビルド可能にする。さらに WASM API の実装と Node.js 上での動作確認まで行う。

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

`lgcell2-core` は wasm ライブラリとしても使用される想定だが、以下の問題がある。

1. `clap` が `[dependencies]` に含まれており、wasm32 ターゲットでコンパイルできない
2. WASM 向けの API レイヤー（`wasm-bindgen` によるエクスポート）が未実装
3. WASM ビルドのスクリプト・手順が存在しない
4. ビルド結果の動作確認手段がない

`local/nospace20` で実績のあるパターンに倣い、feature flag による分離と WASM 対応を段階的に実装する。

## 設計方針

nospace20 と同様に **feature flag パターン** を採用する。

- `cli` feature: clap 等の CLI 専用依存を含む（default）
- `wasm` feature: wasm-bindgen 等の WASM 専用依存を含む
- バイナリには `required-features = ["cli"]` を設定
- `crate-type = ["cdylib", "rlib"]` で WASM・ネイティブ両対応

workspace 化（案 A）は構造変更が大きいため見送り、feature flag（案 B 拡張版）を採用する。

## フェーズ構成

| フェーズ | 内容 | ドキュメント |
|---------|------|------------|
| 1 | feature flag による clap 依存分離 | [01-feature-flags.md](01-feature-flags.md) |
| 2 | WASM API レイヤー実装 | [02-wasm-api.md](02-wasm-api.md) |
| 3 | WASM ビルドスクリプト整備 | [03-wasm-build.md](03-wasm-build.md) |
| 4 | Node.js 動作確認テスト | [04-wasm-test.md](04-wasm-test.md) |

フェーズ 1 は独立して実装可能。フェーズ 2〜4 は順序依存がある。
