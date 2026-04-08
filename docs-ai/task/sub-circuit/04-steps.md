# 実装ステップ

## ステップ 1: データモデルの追加 ✅

### 対象ファイル
- `src/circuit/module.rs`（新規）: `ResolvedModule` 型
- `src/circuit/mod.rs`: `module` モジュールの追加、re-export
- `src/base/error.rs`: サブ回路関連のエラーバリアント追加

### 作業内容
1. ✅ `ResolvedModule` 構造体を定義（アクセサのみ、ロジックなし）
2. ✅ `CircuitError` に `ModuleOutputHasIncomingWires`, `DuplicateModuleOutput`, `ModuleOutputBeforeInput`, `InvalidPortColumn`, `SubInputCountMismatch`, `SubOutputCountMismatch`, `SubOutputBeforeSubInput`, `SubInputHasIncomingWires` を追加
3. ✅ `ParseError` に `SubCircuitNotFound`, `CircularDependency` を追加

## ステップ 2: Circuit の拡張 ✅

### 対象ファイル
- `src/circuit/circuit.rs`: `modules` フィールドと `with_modules` コンストラクタ

### 作業内容
1. ✅ `Circuit` に `modules: Vec<ResolvedModule>` フィールドを追加
2. ✅ `Circuit::with_modules()` コンストラクタを追加（既存 `with_components` を拡張）
3. ✅ モジュール出力セルの検証ロジック（入力ワイヤ禁止、重複禁止、ポート列制約）
4. ✅ 既存テストが通ることを確認（`modules` が空の場合に既存動作が変わらない）

## ステップ 3: JSON パース（サブ回路解決） ✅

### 対象ファイル
- `src/parser/json.rs`: `SubCircuitJson`, `ModuleJson`, `CircuitJson` 拡張、解決ロジック
- `src/parser/json_tests.rs`: サブ回路パースのテスト
- `src/circuit/builder.rs`: `modules` フィールドと `add_module()` 追加
- `src/circuit/circuit.rs`: `validate_port_column_public()` 追加

### 作業内容
1. ✅ `SubCircuitJson`, `ModuleJson` 構造体を追加
2. ✅ `CircuitJson` に `modules`, `subs` フィールドを追加
3. ✅ 循環依存検出（トポロジカルソート / Kahn's algorithm）の実装
4. ✅ サブ回路定義の再帰的な `Circuit` 構築（`build_sub_circuit`, `resolve_module`）
5. ✅ `ModuleJson` → `ResolvedModule` 変換（カウント検証含む）
6. ✅ `TryFrom<CircuitJson> for Circuit` の更新
7. ✅ テスト 10 件追加（正常系: 単一モジュール, シミュレーション付きインバータ, 半加算器, ネストモジュール, 後方互換性 / 異常系: サブ回路未定義, 循環依存, 入出力カウント不一致, sub_input 入力ワイヤ禁止）

### 修正した不具合
- トポロジカルソートで同じサブ回路を複数回参照すると依存カウントが重複し、`CircularDependency` の誤検出が発生 → 依存リストの重複排除で修正

## ステップ 4: シミュレーションエンジンの変更 ✅

### 対象ファイル
- `src/simulation/engine_simple.rs`: `SimulatorSimple` の拡張
- `src/simulation/engine_tests.rs`: モジュール付きシミュレーションテスト

### 作業内容
1. ✅ `sub_simulators: Vec<SimulatorSimple>` フィールド追加
2. ✅ `module_output_cells: HashSet<usize>`, `module_triggers: HashMap<usize, usize>` の事前計算
3. ✅ `step()` メソッドの修正（トリガーチェック + 出力セルスキップ）
4. ✅ `evaluate_module()` の実装（`cell_values`/`prev_cell_values` で入力注入、`tick()` で実行、出力値を親に反映）
5. ✅ `with_output_format()` での子 `SimulatorSimple` 再帰構築
6. ✅ テスト 6 件追加（インバータ false→true / true→false, 出力が後続セルに伝搬, ジェネレータ入力, 2モジュール共有入力, 空モジュール後方互換性）

### 修正した不具合
- モジュール出力セルが `Circuit::with_components()` のワイヤ・エンドポイント検証前に `cells` に含まれていなかったため `WireSrcNotFound` が発生 → `with_modules()` でセル挿入を先に行うよう修正

## ステップ 5: WASM API の対応

詳細: [06-wasm-api.md](06-wasm-api.md)

### 対象ファイル
- `src/wasm_api/types.rs`: `WasmModuleInput`, `WasmSubCircuitInput` 追加、`WasmCircuitInput` 拡張
- `src/wasm_api/simulator.rs`: `build_circuit_from_input()` の更新（`WasmCircuitInput` → `CircuitJson` 変換）

### 作業内容
1. `WasmModuleInput`, `WasmSubCircuitInput` 型を追加
2. `WasmCircuitInput` に `modules`, `sub_circuits` フィールドを追加
3. `build_circuit_from_input()` を更新: `WasmCircuitInput` → `CircuitJson` に変換し、既存の `TryFrom<CircuitJson>` を経由
4. Legacy API（`simulate`, `simulate_n`）は `parse_circuit_json()` 経由のため変更不要

## ステップ 6: View モードのエラーハンドリング

詳細: [05-view.md](05-view.md)

### 対象ファイル
- `src/bin/lgcell2/view.rs`: `run_view_mode()` にサブ回路チェック追加

### 作業内容
1. `run_view_mode()` の先頭で `circuit.modules().is_empty()` を検査
2. サブ回路を含む場合はエラーメッセージを返す

## ステップ 7: テスト

### 対象ファイル
- `src/circuit/module_tests.rs`（新規）: ResolvedModule のユニットテスト
- `src/circuit/circuit_tests.rs`: モジュール付き Circuit の構築テスト
- `src/io/json_tests.rs`: サブ回路 JSON のパーステスト
- `src/simulation/engine_tests.rs`: モジュール付きシミュレーションテスト
- `src/wasm_api/simulator.rs`: サブ回路付きの WASM API テスト
- `resources/tests/simulation/`（新規テストケース）

### テストケース

**正常系:**
- 単一モジュール（半加算器）の組合せ回路
- 複数モジュール（同じサブ回路を 2 回インスタンス化）
- ネストモジュール（全加算器 = 半加算器 × 2）
- 順序回路サブ回路（フィードバックを含むサブ回路）
- 入力セル共有（2 つのモジュールが同じ入力セルを参照）
- サブ回路なしの回路（後方互換性）
- WASM API: `new()` でサブ回路付き `WasmCircuitInput` からの構築
- WASM API: `from_json()` でサブ回路付き JSON からの構築
- WASM API: サブ回路付き回路での `run()`, `run_steps()`, `get_state()`, `get_cell()`

**異常系:**
- 存在しないサブ回路名を参照
- 循環依存（A → B → A）
- 入出力カウント不一致
- 出力セル座標が入力より前
- 出力セルに入力ワイヤが接続
- 出力セルが複数モジュール間で重複
- sub_input に入力ワイヤが接続
- ポート列制約違反（入力ポートが異なる x 座標）
- ポート列制約違反（y 座標が非連続）
- View モードでサブ回路を含む回路を拒否
- WASM API: 不正なサブ回路定義でのエラー

## ステップ 8: ドキュメント更新

### 対象ファイル
- `docs/spec/circuit-json.md`: JSON 仕様にサブ回路セクション追加
- `docs-ai/architecture/data-model.md`: ResolvedModule の記述追加
- `docs-ai/architecture/simulation-model.md`: 階層的シミュレーションの記述追加
- `docs-ai/architecture/circuit-examples.md`: サブ回路を使った回路例の追加
