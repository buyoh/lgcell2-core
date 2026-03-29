# WASM ビルドとテスト

lgcell2-core は feature flag により WASM ターゲットへのビルドに対応しています。

## 前提条件

以下のツールが必要です。

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Node.js（動作確認テスト用）も必要です。

## Feature flag 構成

| Feature | 内容 | デフォルト |
|---------|------|-----------|
| `cli` | clap 等の CLI 専用依存 | 有効 |
| `wasm` | wasm-bindgen / serde-wasm-bindgen | 無効 |

- `cargo build` / `cargo test` は `cli` feature が有効な状態で動作します（従来通り）。
- WASM ビルドでは `--no-default-features --features wasm` を指定し、CLI 依存を除外します。

## ビルド

### ビルドスクリプト

```bash
./build-wasm.sh
```

release / debug 両方のパッケージを生成します。debug ビルドをスキップする場合は:

```bash
NO_DEBUG=true ./build-wasm.sh
```

### 出力先

| ディレクトリ | ビルド種別 |
|-------------|-----------|
| `pkg/` | release（wasm-opt 最適化済み） |
| `pkg-dev/` | debug |

いずれも `.gitignore` で除外されています。

### 手動ビルド

```bash
# release
cargo build --release \
  --target wasm32-unknown-unknown --lib \
  --no-default-features --features wasm
wasm-pack build --target bundler --no-default-features --features wasm

# debug
cargo build \
  --target wasm32-unknown-unknown --lib \
  --no-default-features --features wasm
wasm-pack build --dev --out-dir pkg-dev --target bundler --no-default-features --features wasm
```

出力形式は `--target bundler`（ES Modules）です。

## WASM API

`src/wasm_api.rs` で定義されるエクスポート関数:

### `simulate(circuit_json: string, ticks: bigint): string`

回路 JSON を受け取り、シミュレーション結果 JSON を返します。`ticks` は `BigInt` で渡します。

- `circuit_json` — 回路定義 JSON 文字列（[回路 JSON 仕様](spec/circuit-json.md) 参照）
- `ticks` — シミュレーションする tick 数（`BigInt`）
- 戻り値 — シミュレーション結果 JSON 文字列
- エラー時は例外をスローします

```javascript
const result = simulate(circuitJson, 3n);
const parsed = JSON.parse(result);
// parsed.ticks: Array — 各 tick のセル状態
// parsed.ticks[0].tick === 0
```

### `simulate_n(circuit_json: string, ticks: number): string`

`simulate` と同じ機能ですが、`ticks` を JavaScript の `number`（`u32` 範囲）で渡せます。

```javascript
const result = simulate_n(circuitJson, 3);
const parsed = JSON.parse(result);
```

## テスト

### Rust ユニットテスト

```bash
# wasm feature 有効で Rust テストを実行
cargo test --no-default-features --features wasm --lib
```

### Node.js 動作確認テスト

WASM ビルド後に実行します。

```bash
./build-wasm.sh
node tools/wasm-test/test.mjs
```

テスト内容:

- 基本的なシミュレーション実行（tick 数の検証）
- 不正 JSON 入力でエラーが返ること
- 不正ワイヤ種別でエラーが返ること
