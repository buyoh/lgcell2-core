# 実装ステップ

## ステップ 1: データモデルの追加

### 対象ファイル
- `src/circuit/module.rs`（新規）: `ResolvedModule` 型
- `src/circuit/mod.rs`: `module` モジュールの追加、re-export
- `src/base/error.rs`: サブ回路関連のエラーバリアント追加

### 作業内容
1. `ResolvedModule` 構造体を定義（アクセサのみ、ロジックなし）
2. `CircuitError` に `ModuleOutputHasIncomingWires`, `DuplicateModuleOutput`, `ModuleOutputBeforeInput`, `InvalidPortColumn`, `SubInputCountMismatch`, `SubOutputCountMismatch`, `SubOutputBeforeSubInput`, `SubInputHasIncomingWires` を追加
3. `ParseError` に `SubCircuitNotFound`, `CircularDependency` を追加

## ステップ 2: Circuit の拡張

### 対象ファイル
- `src/circuit/circuit.rs`: `modules` フィールドと `with_modules` コンストラクタ

### 作業内容
1. `Circuit` に `modules: Vec<ResolvedModule>` フィールドを追加
2. `Circuit::with_modules()` コンストラクタを追加（既存 `with_components` を拡張）
3. モジュール出力セルの検証ロジック（入力ワイヤ禁止、重複禁止、ポート列制約）
4. 既存テストが通ることを確認（`modules` が空の場合に既存動作が変わらない）

## ステップ 3: JSON パース（サブ回路解決）

### 対象ファイル
- `src/io/json.rs`: `SubCircuitJson`, `ModuleJson`, `CircuitJson` 拡張、解決ロジック

### 作業内容
1. `SubCircuitJson`, `ModuleJson` 構造体を追加
2. `CircuitJson` に `modules`, `sub_circuits` フィールドを追加
3. 循環依存検出（トポロジカルソート）の実装
4. サブ回路定義の再帰的な `Circuit` 構築
5. `ModuleJson` → `ResolvedModule` 変換（カウント検証含む）
6. `TryFrom<CircuitJson> for Circuit` の更新

## ステップ 4: シミュレーションエンジンの変更

### 対象ファイル
- `src/simulation/engine.rs`: `Simulator` の拡張

### 作業内容
1. `sub_simulators: Vec<Simulator>` フィールド追加
2. `module_output_cells: HashSet<usize>`, `module_triggers: HashMap<usize, usize>` の事前計算
3. `step()` メソッドの修正（トリガーチェック + 出力セルスキップ）
4. `evaluate_module()` の実装（`set_cell()` で入力注入、`tick()` で実行、`get_cell()` で出力取得）
5. `Simulator::new()` / `with_output_format()` での子 Simulator 再帰構築
6. `WireSimState` との相互作用の確認（sub_input セルが入力なしセルスロットとして扱われることの検証）

## ステップ 5: テスト

### 対象ファイル
- `src/circuit/module_tests.rs`（新規）: ResolvedModule のユニットテスト
- `src/circuit/circuit_tests.rs`: モジュール付き Circuit の構築テスト
- `src/io/json_tests.rs`: サブ回路 JSON のパーステスト
- `src/simulation/engine_tests.rs`: モジュール付きシミュレーションテスト
- `resources/tests/simulation/`（新規テストケース）

### テストケース

**正常系:**
- 単一モジュール（半加算器）の組合せ回路
- 複数モジュール（同じサブ回路を 2 回インスタンス化）
- ネストモジュール（全加算器 = 半加算器 × 2）
- 順序回路サブ回路（フィードバックを含むサブ回路）
- 入力セル共有（2 つのモジュールが同じ入力セルを参照）
- サブ回路なしの回路（後方互換性）

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

## ステップ 6: ドキュメント更新

### 対象ファイル
- `docs/spec/circuit-json.md`: JSON 仕様にサブ回路セクション追加
- `docs-ai/architecture/data-model.md`: ResolvedModule の記述追加
- `docs-ai/architecture/simulation-model.md`: 階層的シミュレーションの記述追加
- `docs-ai/architecture/circuit-examples.md`: サブ回路を使った回路例の追加
