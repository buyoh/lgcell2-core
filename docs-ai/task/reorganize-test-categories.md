# テストリソースのカテゴリ再編成

テストリソース (`resources/tests/`) のディレクトリ構造と分類名を整理し、テストの目的が一目で分かるようにする。

作成日: 2026-03-25
ステータス: 設計完了（未実装）

## 背景・動機

現在のテストリソースには以下の問題がある:

1. **`validation` タイプの名前が不明確**: 回路JSONの解析時エラー検知テストであることが名前から分かりにくい
2. **`simulation` タイプが平坦すぎる**: 単一機能テスト（ワイヤ伝搬、セル保持など）と総合回路テスト（全加算器、JKフリップフロップなど）が同一ディレクトリに混在しており、テストの目的を区別できない

### 現状のディレクトリ構造

```
resources/tests/
  test-manifest.yaml
  simulation/          # 12テスト（全種混在）
    eight_directions/
    feedback_oscillator/
    full_adder/
    generator_pattern/
    half_adder/
    i32_extreme_coords/
    isolated_cell_retains/
    jk_flipflop/
    mixed_polarity_fan_in/
    negative_coordinates/
    shift_register/
    sr_latch/
  validation/          # 8テスト（名前が不明確）
    duplicate_wire_diff_kind/
    duplicate_wire_same_kind/
    generator_duplicate_target/
    generator_empty_pattern/
    generator_incoming/
    generator_invalid_pattern_char/
    self_loop/
    unknown_wire_kind/
```

## 設計・方針

### 1. `validation` → `parse_error` へリネーム

テストの目的が「回路JSON解析時のエラー検知」であることを明確にするため、`type` フィールドとディレクトリ名を `parse_error` に変更する。

内部のテストケースはそのまま維持（サブディレクトリ化は行わない）。

### 2. `simulation` のサブディレクトリ分割

`simulation/` 配下にサブディレクトリを作成し、テストの目的ごとに分類する。`type` フィールドは `simulation` のまま変更しない（テストランナーの挙動は同一のため）。`path` フィールドのみ変更する。

| サブカテゴリ | 目的 | 配置するテスト |
|---|---|---|
| `wire/` | ワイヤの伝搬・極性・フィードバック動作 | `eight_directions`, `feedback_oscillator`, `mixed_polarity_fan_in` |
| `cell/` | セル単体の挙動（値保持など） | `isolated_cell_retains` |
| `generator/` | ジェネレーター機能の動作確認 | `generator_pattern` |
| `boundary/` | 座標の境界条件 | `i32_extreme_coords`, `negative_coordinates` |
| `example/` | 実用回路の総合テスト | `half_adder`, `full_adder`, `sr_latch`, `jk_flipflop`, `shift_register` |

### 3. 新しいディレクトリ構造

```
resources/tests/
  test-manifest.yaml
  parse_error/                         # 旧 validation/
    self_loop/
    duplicate_wire_same_kind/
    duplicate_wire_diff_kind/
    unknown_wire_kind/
    generator_incoming/
    generator_duplicate_target/
    generator_empty_pattern/
    generator_invalid_pattern_char/
  simulation/
    wire/                              # ワイヤ動作テスト
      eight_directions/
      feedback_oscillator/
      mixed_polarity_fan_in/
    cell/                              # セル動作テスト
      isolated_cell_retains/
    generator/                         # ジェネレーターテスト
      generator_pattern/
    boundary/                          # 境界条件テスト
      i32_extreme_coords/
      negative_coordinates/
    example/                           # 総合回路テスト
      half_adder/
      full_adder/
      sr_latch/
      jk_flipflop/
      shift_register/
```

### 4. コード変更箇所

#### `test-manifest.yaml`

- `type: validation` → `type: parse_error` に変更
- `path` フィールドをサブディレクトリ付きに更新（例: `simulation/half_adder` → `simulation/example/half_adder`）
- コメント中のセクション見出しを新分類に合わせて更新

#### `build.rs`

- `match` 文の `"validation"` アームを `"parse_error"` に変更

```rust
// 変更前
"validation" => write_validation_test(&mut f, &test),
// 変更後
"parse_error" => write_validation_test(&mut f, &test),
```

関数名 `write_validation_test` や `test_validation_case` はテストランナーの内部実装名であり、外部から参照されないため変更不要。

#### `tests/test_helpers.rs`

変更不要。`test_simulation_case` / `test_validation_case` は `path` 文字列を受け取ってファイルを読むだけなので、`type` 名には依存しない。

#### ディレクトリ移動

実際のテストデータファイル（`circuit.json`, `check.json`, `expected.json`）のディレクトリを移動する。ファイル内容の変更は不要。

## ステップ

1. `resources/tests/simulation/` 配下にサブディレクトリ (`wire/`, `cell/`, `generator/`, `boundary/`, `example/`) を作成
2. 各テストケースディレクトリを対応するサブディレクトリに移動
3. `resources/tests/validation/` を `resources/tests/parse_error/` にリネーム
4. `test-manifest.yaml` を更新（`type`, `path`, コメント）
5. `build.rs` の `"validation"` → `"parse_error"` を変更
6. `cargo test` で全テスト通過を確認
