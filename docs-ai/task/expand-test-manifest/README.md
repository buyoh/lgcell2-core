# test-manifest テストケース拡充

test-manifest に多数のテストケースを追加し、テスト基盤の拡張を行う。

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

現在 test-manifest には `half_adder` の 4 ケースしか存在しない。回路エディタ・シミュレータとしての品質を担保するために、以下の観点でテストを拡充する必要がある。

- **機能テスト**: 8 方向のワイヤ配置、フィードバック（遅延伝搬）の動作確認など
- **境界テスト**: 座標値の i32 範囲での動作保証
- **失敗テスト**: 不正な回路（多重辺、self-loop 等）の拒否確認
- **総合テスト**: JK フリップフロップ、カウンターなどの実用回路

総合テスト（順序回路）では tick ごとに入力パターンを変化させる必要があるため、新しい端子「ジェネレーター」の導入が前提となる。ジェネレーターは回路モデルの一部として設計し、通常の回路でも使用できる。

## 設計ドキュメント

| ドキュメント | 内容 |
|---|---|
| [01-generator.md](01-generator.md) | ジェネレーター端子の設計 |
| [02-test-cases.md](02-test-cases.md) | 追加するテストケースの一覧と回路定義 |

## ステップ

### Phase 1: ジェネレーター機能の実装

1. **Generator / GeneratorMode 型の追加** — `src/circuit/generator.rs` に `Generator`, `GeneratorMode` を定義
2. **Circuit 拡張** — `with_generators()` コンストラクタ、ジェネレーターバリデーション追加
3. **circuit.json スキーマ拡張** — `generators` フィールドのパース（パターン文字列 `"101"` 形式、mode `"hold"` / `"loop"`）
4. **Simulator 拡張** — tick 開始時のジェネレーター値自動適用

### Phase 2: テスト基盤の拡張

5. **check.json スキーマ拡張** — per-case ジェネレーター、per-case `ticks` オーバーライド
6. **テストランナー修正** (`tests/test_helpers.rs`) — circuit.json/check.json ジェネレーターのマージ
7. **build.rs 拡張** — `type: validation` テストのコード生成対応
8. **test_helpers.rs 拡張** — `test_validation_case()` 関数の追加

### Phase 3: テストデータの追加

9. **機能テスト** — 8 方向ワイヤ、フィードバック動作、混合極性、孤立セル保持
10. **境界テスト** — i32 極値座標、負座標、広範囲座標
11. **失敗テスト** — self-loop、多重辺、不正 kind
12. **総合テスト** — SR ラッチ、JK フリップフロップ、カウンター、全加算器

### Phase 4: 既存テストの整理

13. **既存テストの検討** — `engine_tests.rs` の一部テストを manifest 形式に変換するか検討（基本ゲートの真理値表テスト等）
