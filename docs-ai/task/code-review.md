# コードレビュー: src/ 以下のソースコード

**対象:** `src/` 以下の全ソースコード  
**日付:** 2026-03-23  
**ステータス:** レビュー完了・修正待ち

---

## 1. clap 依存が library crate 全体に含まれている

**重要度:** high  
**対象:** [Cargo.toml](../../Cargo.toml)

### 問題点

`clap` は `src/bin/lgcell2/main.rs` でのみ使用されるが、`[dependencies]` に含まれているため library crate 全体の依存となっている。  
`lgcell2-core` は wasm ライブラリとしても使用される想定だが、`clap` は wasm32 ターゲットではコンパイルできず、wasm ビルドが失敗する。

### 違反するルール・原則

- wasm ターゲットに対応する設計要件に反する
- 依存関係の最小化原則（不要な依存をライブラリ利用者に強制しない）

### 解決策

**案 A: バイナリを別 crate に分離 (推奨)**  
`lgcell2` バイナリを workspace member として分離し、`clap` はそちらの `Cargo.toml` に記載する。

- 影響範囲: `Cargo.toml`, ディレクトリ構造の変更
- 利点: ライブラリの依存が完全にクリーンになる
- 欠点: workspace 化の作業が必要

**案 B: feature flag で分離**  
`clap` を optional dependency にし、binary に `required-features` を設定する。

- 影響範囲: `Cargo.toml` のみ
- 利点: 構造変更が最小限
- 欠点: feature flag の管理が必要

---

## 2. SRP 違反: `io/json.rs` にシミュレーション実行ロジックが含まれる

**重要度:** medium  
**対象:** [src/io/json.rs](../../src/io/json.rs) — `simulate_to_output_json` 関数

### 問題点

`simulate_to_output_json` 関数は I/O モジュールに配置されているが、以下の 3 つの責務を持っている:

1. シミュレーションの実行（`Simulator::tick()` の呼び出し）
2. 結果の収集（状態の取得・ソート）
3. JSON 出力モデルへの変換

I/O モジュールの責務は「データの読み書き・変換」であり、シミュレーション実行のオーケストレーションは含まれるべきでない。

### 違反するルール・原則

- copilot-instructions: 「単一責任原則に従い、モジュール・構造体を分割する」
- SRP (Single Responsibility Principle)

### 解決策

**案 A: シミュレーション結果収集を `simulation` モジュールに移動 (推奨)**  
`Simulator` に tick 結果を `Vec` として返すメソッドを追加し、`io/json.rs` は結果の JSON 変換のみを担当する。

- 影響範囲: `simulation/engine.rs`, `io/json.rs`, `main.rs`

**案 B: 別のオーケストレーション層を導入**  
`run` モジュール等を新設し、シミュレーション実行 + 結果収集のロジックをそこに配置する。

- 影響範囲: 新規モジュール追加、`io/json.rs`, `main.rs`

---

## 3. モジュール名と内容の不一致: `cell.rs` に `Pos` のみ

**重要度:** medium  
**対象:** [src/circuit/cell.rs](../../src/circuit/cell.rs)

### 問題点

モジュール名 `cell` は「セル」概念を表すが、実際に定義されているのは `Pos`（座標）のみ。  
セルの概念は `Circuit` 内で `BTreeMap<Pos, bool>` として暗黙的に表現されており、明示的な `Cell` 型が存在しない。

これにより:
- モジュール名から内容を推測できない
- セルに属性（ラベル、種別等）を追加する際、`Circuit` の内部表現から変更が必要になる

### 違反するルール・原則

- 命名の一貫性（モジュール名と内容の対応）
- 関心の分離（セルの概念がデータ構造に埋め込まれている）

### 解決策

**案 A: ファイル名を `pos.rs` にリネーム**  
現状の内容に即した名前に変更する。`mod.rs` の `pub mod` も修正。

- 影響範囲: `cell.rs` → `pos.rs`, `cell_tests.rs` → `pos_tests.rs`, `mod.rs`
- 利点: 最小限の変更で名前と内容が一致する
- 欠点: 将来 Cell 抽象が必要になった際に再度リネームが必要

**案 B: `Cell` 構造体を導入**  
`Pos` に加えて `Cell { pos: Pos, initial: bool }` を定義し、`Circuit` の内部表現を `BTreeMap<Pos, bool>` から `Vec<Cell>` 等に変更する。

- 影響範囲: `cell.rs`, `circuit.rs`, `state.rs`, `io/json.rs`, テスト全般
- 利点: セルが第一級の概念になり、拡張性が向上する
- 欠点: 影響範囲が広く、既存 API の破壊的変更となる

---

## 4. 自己ループ検証の重複

**重要度:** low  
**対象:** [src/io/json.rs](../../src/io/json.rs) `TryFrom<CircuitJson>`, [src/circuit/circuit.rs](../../src/circuit/circuit.rs) `Circuit::new()`

### 問題点

自己ループ (`src == dst`) のバリデーションが `TryFrom<CircuitJson>` と `Circuit::new()` の両方で実行されている。  
`TryFrom` は最終的に `Circuit::new()` を呼び出すため、`TryFrom` 側の検証は冗長。

### 違反するルール・原則

- DRY (Don't Repeat Yourself)
- バリデーションは一つの場所で行うべき

### 解決策

`TryFrom<CircuitJson>` の自己ループ検証を削除し、`Circuit::new()` に一元化する。

- 影響範囲: `io/json.rs` のみ（`TryFrom` 実装から self-loop チェックを削除）
- テスト: `json_tests::parse_rejects_self_loop` は `Circuit::new()` 経由で引き続きパスする

---

## 5. `simulate_to_output_json` 内の冗長なソート

**重要度:** low  
**対象:** [src/io/json.rs](../../src/io/json.rs) — `simulate_to_output_json` 内

### 問題点

毎 tick ごとに `simulator.state().values().keys()` を取得してソートしているが、`Circuit` は内部に `sorted_cells` を保持済み。  
ただし、`Simulator` から `Circuit` の `sorted_cells()` にアクセスする公開 API がないため、現状は冗長なソートを行っている。

### 違反するルール・原則

- 不要な計算の回避

### 解決策

`Simulator` に `circuit()` アクセサを追加し、`circuit.sorted_cells()` を利用する。

- 影響範囲: `simulation/engine.rs`（アクセサ追加）, `io/json.rs`（ソートの置き換え）

---

## 6. エラー型に `String` を使用

**重要度:** low  
**対象:** 全モジュール (`circuit.rs`, `state.rs`, `io/json.rs`, `main.rs`)

### 問題点

全てのエラーハンドリングで `String` をエラー型として使用している。  
ライブラリ crate として公開する場合、利用者がエラーの種類に応じた処理を行うことができない。

### 違反するルール・原則

- Rust のエラーハンドリングのベストプラクティス（構造化されたエラー型の使用）
- ライブラリの使いやすさ

### 解決策

**案 A: `thiserror` による enum エラー型の導入 (推奨)**  
`circuit::CircuitError`, `simulation::SimError` 等の enum を定義し、`String` を置き換える。

- 影響範囲: 全モジュール、全テスト
- 利点: エラーのパターンマッチが可能になり、ライブラリとしての品質が向上
- 欠点: 変更範囲が広い

**案 B: 現状維持**  
プロジェクト初期段階であり、エラーパターンが安定するまで `String` で運用する。

- 利点: 変更不要
- 欠点: API 安定後の移行コストが増大する
