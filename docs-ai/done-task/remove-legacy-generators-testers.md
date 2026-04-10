# generators/testers 後方互換の削除

回路 JSON パーサーから `generators` / `testers` フィールドの後方互換処理を削除する。

作成日: 2026-04-10
ステータス: 完了

## 背景・動機

回路 JSON の `input` / `output` フィールドへの移行が完了し、`generators` / `testers` フィールドの後方互換を維持する必要がなくなった。コードの簡素化のため、レガシーフィールドのサポートを削除する。

## 設計・方針

- `CircuitJson` 構造体から `generators` / `testers` フィールドを削除
- `GeneratorJson` / `TesterJson` 構造体（レガシー用）を削除
- `TryFrom<CircuitJson>` からレガシー処理ループを削除
- `input` / `output` フィールドは引き続き利用可能
- JSON に `generators` / `testers` が含まれていても serde がサイレントに無視する（エラーにはならない）

## 変更ファイル

- `src/parser/json.rs`: メインパーサーからレガシーフィールド・構造体・処理を削除
- `src/io/json.rs`: IO パーサーからレガシーフィールド・構造体・処理を削除
- `src/parser/json_tests.rs`: レガシーテストを、フィールドが無視されることを検証するテストに置換
- `src/io/json_tests.rs`: 同上
- `tests/test_helpers.rs`: `circuit_json.generators` / `circuit_json.testers` の参照を削除
- `docs/spec/circuit-json.md`: 後方互換に関する注記を削除
