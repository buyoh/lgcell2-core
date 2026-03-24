# エラー型を String から構造化された型に移行

全モジュールのエラーハンドリングで `String` を使用している問題を解決する。

作成日: 2026-03-23
ステータス: 設計完了（未実装）

## 背景・動機

全てのエラーハンドリングで `String` をエラー型として使用している。ライブラリ crate として公開する場合、利用者がエラーの種類に応じた処理（パターンマッチ等）を行うことができない。

重要度: low

## 現状の問題点

以下の関数が `Result<_, String>` を返している:

| 関数 | ファイル | 用途 |
|------|---------|------|
| `Circuit::new()` | `src/circuit/circuit.rs` | 回路トポロジのバリデーション |
| `SimState::set()` | `src/simulation/state.rs` | 存在しないセルへのアクセス |
| `StateMut::set()` | `src/simulation/engine.rs` | SimState::set の委譲 |
| `TryFrom<CircuitJson> for Circuit` | `src/io/json.rs` | JSON→Circuit 変換 |
| `parse_circuit_json()` | `src/io/json.rs` | JSON パース |
| `output_json_to_string()` | `src/io/json.rs` | JSON シリアライズ |
| `read_input()` | `src/bin/lgcell2/main.rs` | ファイル/stdin 読み込み |
| `run()` | `src/bin/lgcell2/main.rs` | CLI エントリポイント |

エラーメッセージは `format!()` マクロによる文字列生成、および `.to_string()` による外部エラーの変換で構成されている。

## 設計・方針

`thiserror` による enum エラー型を `src/base` モジュールに集約する。

### モジュール構成

```
src/
  base/
    mod.rs          # pub mod error; + re-export
    error.rs        # 全エラー型の定義
  circuit/
  io/
  simulation/
  lib.rs            # pub mod base; を追加
```

`base` モジュールは他の全モジュールから参照される基盤モジュールとなる。
`base/error.rs` 内のエラー型は `crate::circuit::Pos` を使用する（同一クレート内なので循環参照の問題はない）。

### エラー型の定義

3 つのエラー enum を定義する。

#### CircuitError — 回路構造のバリデーションエラー

`Circuit::new()` が返すエラー。

```rust
#[derive(Debug, thiserror::Error)]
pub enum CircuitError {
    #[error("self-loop wire is not allowed: src=({}, {}), dst=({}, {})", .src.x, .src.y, .dst.x, .dst.y)]
    SelfLoop { src: Pos, dst: Pos },

    #[error("wire src does not exist in cells: ({}, {})", .0.x, .0.y)]
    WireSrcNotFound(Pos),

    #[error("wire dst does not exist in cells: ({}, {})", .0.x, .0.y)]
    WireDstNotFound(Pos),

    #[error("duplicate wire is not allowed: src=({}, {}), dst=({}, {})", .src.x, .src.y, .dst.x, .dst.y)]
    DuplicateWire { src: Pos, dst: Pos },
}
```

#### ParseError — JSON パース・変換エラー

`parse_circuit_json()` および `TryFrom<CircuitJson>` が返すエラー。
`CircuitError` を `#[from]` で内包し、パース→バリデーションのエラー伝播を `?` で行えるようにする。

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("wire kind must be positive or negative: {0}")]
    InvalidWireKind(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Circuit(#[from] CircuitError),
}
```

#### SimulationError — シミュレーション実行時エラー

`SimState::set()` / `StateMut::set()` が返すエラー。

```rust
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("unknown cell at ({}, {})", .0.x, .0.y)]
    UnknownCell(Pos),
}
```

### バイナリのエラーハンドリング

`src/bin/lgcell2/main.rs` はライブラリ API ではないため、`Box<dyn std::error::Error>` を使用する。

```rust
fn read_input(file: Option<PathBuf>) -> Result<String, std::io::Error> { ... }
fn run() -> Result<(), Box<dyn std::error::Error>> { ... }
```

### output_json_to_string のエラー型

`output_json_to_string()` はシリアライズ専用であり、独自のエラーバリアントは不要。
`serde_json::Error` を直接返すように変更する。

```rust
pub fn output_json_to_string(output: &SimulationOutputJson) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(output)
}
```

### エラーメッセージの互換性

既存のエラーメッセージ文字列はそのまま維持する（`Display` トレイトの出力を一致させる）。
これにより、テストの `.contains()` によるアサーションは引き続き動作する。ただし、構造化エラーへの移行後はパターンマッチによるアサーションに順次書き換える。

## ステップ

1. `Cargo.toml` に `thiserror` 依存を追加
2. `src/base/mod.rs`, `src/base/error.rs` を作成、`src/lib.rs` に `pub mod base;` を追加
3. `src/circuit/circuit.rs`: `Circuit::new()` の戻り値を `Result<Self, CircuitError>` に変更
4. `src/io/json.rs`: `TryFrom` の `Error` を `ParseError` に、`parse_circuit_json()` の戻り値を `Result<Circuit, ParseError>` に変更。`output_json_to_string()` は `Result<String, serde_json::Error>` に変更
5. `src/simulation/state.rs` + `engine.rs`: `set()` の戻り値を `Result<(), SimulationError>` に変更
6. `src/bin/lgcell2/main.rs`: `read_input()` を `Result<String, std::io::Error>` に、`run()` を `Result<(), Box<dyn std::error::Error>>` に変更
7. 全テストファイルのアサーションをパターンマッチに書き換え
8. `tests/circuit_tests.rs`（統合テスト）のエラーアサーションを更新
