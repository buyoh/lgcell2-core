# コードレビュー 2026-03-26

## 概要

ソースコード、テストコード、設定ファイルを対象としたコードレビュー。
`.github/copilot-instructions.md` の注意事項への違反と、一般的な設計原則に反する点を調査。

## 検出された問題

### 1. 回路構築ロジックの重複

- **重要度**: medium
- **違反している原則**: DRY 原則（Don't Repeat Yourself）、単一責任原則

**説明**:
以下の 3 箇所で「ワイヤ列 → cells / wires / generators を構築 → `Circuit::with_generators()` を呼ぶ」という同一パターンのコードが重複している。

1. `src/io/json.rs` — `TryFrom<CircuitJson> for Circuit` (L56–L82)
2. `src/wasm_api/simulator.rs` — `build_circuit_from_input()` (L123–L155)
3. `tests/test_helpers.rs` — `build_circuit_with_case_generators()` (L103–L139)

いずれも以下を実行:
- `BTreeSet<Pos>` を作成
- ワイヤ列を走査して `Wire` を構築、`cells` に `src`/`dst` を挿入
- ジェネレーター列を走査して `Generator` を構築
- `Circuit::with_generators()` を呼ぶ

**解決策の提案**:

**案 A: Circuit にビルダーパターンを導入**
- `CircuitBuilder` 構造体を追加し、`add_wire(src, dst, kind)` / `add_generator(target, pattern, is_loop)` メソッドで構築する
- セルの自動推論（ワイヤ端点からの挿入）をビルダー側に集約
- 影響範囲: `circuit` モジュールに追加、上記 3 箇所を書き換え

**案 B: 変換元の型を統一する中間構造体を用意**
- `CircuitBuilder` ではなく、`(src, dst, kind)` のスライスと `(target, pattern, is_loop)` のスライスを受け取るコンストラクタを `Circuit` に追加
- セルの自動推論を `Circuit` 側に移す
- 影響範囲: `circuit.rs` に関数追加、上記 3 箇所を書き換え

---

### 2. `engine.rs` のプロダクションコードにおける `expect()` の使用

- **重要度**: medium
- **違反している原則**: ライブラリコードで panic を使用すべきでない（堅牢性の原則）

**説明**:
`src/simulation/engine.rs` の `apply_generators()` および `step()` メソッドに計 8 箇所の `.expect()` がある。各 `.expect()` にはコメントで不変条件が説明されており、論理的には到達しないパスではあるが、ライブラリとして外部から利用される場合に panic は不適切。

該当箇所:
- L79, L83: `apply_generators()` 内の `prev_state.set()`, `curr_state.set()`
- L93: `prev_state.get(cell)` — 入力なしセルの値保持
- L96: `curr_state.set(cell, retained)` — 保持値の書き込み
- L101, L105: `prev_state.get(wire.src)`, `curr_state.get(wire.src)` — ワイヤ元の値読み出し
- L114: `curr_state.set(cell, next_value)` — 計算結果の書き込み
- L152: `prev_state.get(*pos)` — スナップショット収集

**解決策の提案**:

**案 A: `step()` の戻り値を `Result<StepResult, SimulationError>` に変更**
- 不変条件違反時に `Err` を返し、呼び出し元で処理する
- 影響範囲: `Simulator::step()`, `tick()`, `run()`, `run_with_snapshots()` の戻り値変更。`wasm_api` と `bin/lgcell2` の呼び出し側も変更

**案 B: 現状維持 + `debug_assert!` による防御**
- `expect()` のメッセージは維持しつつ、`Circuit` 構築時の不変条件が保証されていることをドキュメントで明示
- `debug_assert!` でテスト時のみ検証を追加
- 影響範囲: 最小限（ドキュメントとアサーション追加のみ）

---

### 3. `Circuit::with_generators()` の責務過多

- **重要度**: low
- **違反している原則**: 単一責任原則（SRP）

**説明**:
`src/circuit/circuit.rs` の `Circuit::with_generators()` メソッド（L33–L97）は約 65 行で、以下の 7 つの処理を一体で行っている:

1. セルフループ検出
2. ワイヤ src の存在検証
3. ワイヤ dst の存在検証
4. 重複ワイヤ検出
5. `incoming` インデックスマップ構築
6. ジェネレーター検証（入力ワイヤ禁止、重複ターゲット禁止、空パターン禁止）
7. `sorted_cells` リスト生成

現時点では可読性に問題はないが、今後バリデーションルールが増える場合にメンテナンス性が低下する可能性がある。

**解決策の提案**:

**案 A: バリデーションを内部関数に分割**
- `validate_wires()`, `validate_generators()`, `build_indices()` のような private 関数に分割
- `with_generators()` はこれらを順に呼び出すオーケストレータに
- 影響範囲: `circuit.rs` 内のリファクタリングのみ。公開 API に変更なし

**案 B: 現状維持**
- 現在の規模（65 行）は許容範囲内であり、分割によるオーバーヘッド（複数関数間の引数受け渡し）の方が大きい
- 影響範囲: なし

---

### 4. `build.rs` のエラーハンドリング不統一

- **重要度**: low
- **違反している原則**: コードの一貫性

**説明**:
`build.rs` 内で `.unwrap()` と `.expect()` が混在している:

| 行 | パターン | 内容 |
|---|---|---|
| L36 | `.expect()` | test-manifest.yaml の読み込み |
| L38 | `.expect()` | YAML パース |
| L40 | `.unwrap()` | `OUT_DIR` 環境変数の取得 |
| L41 | `.unwrap()` | ファイル作成 |
| L63, L76 | `.unwrap_or_else(\|_\| panic!())` | check.json の読み込みとパース |
| L88, L108 | `.unwrap()` | `writeln!` の結果 |

**解決策の提案**:

`.unwrap()` を全て `.expect("説明")` に統一する。ビルドスクリプトでは panic が許容されるが、エラーメッセージの有無は開発者体験に影響する。

影響範囲: `build.rs` のみ
