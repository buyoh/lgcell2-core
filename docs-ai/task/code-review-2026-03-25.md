# コードレビュー 2026-03-25

`src/` および `tests/` 以下のコードレビュー結果。

作成日: 2026-03-25
ステータス: 未着手

---

## 1. ParseError::InvalidWireKind がパターン解析エラーに流用されている

重要度: **medium**

### 問題点

`io/json.rs` の `parse_pattern` 関数で、不正なパターン文字（`'0'` `'1'` 以外）が検出された際に `ParseError::InvalidWireKind` を返している。ジェネレーターのパターン解析エラーはワイヤ種別の不正とは無関係であり、エラーバリアント名と実際のエラー内容が乖離している。

同じ問題が `tests/test_helpers.rs` の `parse_pattern` にも存在する。

### 違反しているルール・原則

- 単一責任原則: 1 つのエラーバリアントが複数の意味を持っている
- 命名規約: バリアント名がエラーの性質を正しく表現していない
- 可読性・保守性: エラーハンドリング時にパターンエラーとワイヤ種別エラーを区別できない

### 該当箇所

- [src/io/json.rs](../../src/io/json.rs#L91-L99) `parse_pattern` 関数
- [tests/test_helpers.rs](../../tests/test_helpers.rs#L120-L130) `parse_pattern` 関数

### 解決策

`ParseError` に `InvalidPatternChar(char)` バリアントを追加し、パターン解析エラー専用にする。

```rust
pub enum ParseError {
    #[error("invalid pattern character: '{0}' (expected '0' or '1')")]
    InvalidPatternChar(char),
    // ...existing variants...
}
```

### 影響範囲

- `src/base/error.rs`: バリアント追加
- `src/io/json.rs`: `parse_pattern` のエラー返却を変更
- `tests/test_helpers.rs`: 同上
- `src/io/json_tests.rs`: `parse_rejects_invalid_generator_pattern_char` テストの `matches!` パターン修正

---

## ~~2. tests/test_helpers.rs と io/json.rs にコードが重複している~~ (対応済み)

重要度: **medium**

### 問題点

`tests/test_helpers.rs` に `parse_wire_kind` と `parse_pattern` が `io/json.rs` のものとほぼ同一のロジックで重複定義されている。変更が一方にしか反映されないリスクがある。

### 違反しているルール・原則

- DRY 原則（Don't Repeat Yourself）: 同一ロジックが 2 箇所に存在
- 保守性: 一方を修正した際にもう一方への修正漏れが発生し得る

### 該当箇所

- [src/io/json.rs](../../src/io/json.rs#L63-L99) `TryFrom<CircuitJson> for Circuit` 内の wire kind 変換と `parse_pattern`
- [tests/test_helpers.rs](../../tests/test_helpers.rs#L108-L130) `parse_wire_kind` と `parse_pattern`

### 解決策

**案 A**: `io/json.rs` の `parse_pattern` と wire kind 変換のロジックを pub 関数として公開し、`test_helpers.rs` から呼び出す。

- 影響範囲: `src/io/json.rs`（公開範囲変更）, `tests/test_helpers.rs`（重複削除・呼び出しに変更）

**案 B**: wire kind/pattern のパースを `circuit` モジュール側に移動して共通化する。

- 影響範囲: `src/circuit/wire.rs` or 新モジュール追加, `src/io/json.rs`, `tests/test_helpers.rs`
- io 層の責務が circuit 層に漏れるため、案 A の方が適切

### 推奨

案 A が最小変更で済む。`parse_pattern` を `pub fn` にし、`WireKind` に `FromStr` を実装するか、wire kind 変換を pub 関数化する。

---

## ~~3. Generator::value_at で u64 → usize へのキャストが wasm32 で切り捨てを起こす~~ (対応済み)

重要度: **low**

### 問題点

`generator.rs` の `value_at` メソッドで `tick as usize` としてキャストしているが、wasm32 ターゲットでは `usize` が 32 ビットであるため、`tick > u32::MAX` の場合に上位ビットが切り捨てられ、誤った値が返る。

- ループモード: `(tick % 2^32) % pattern.len()` ≠ `tick % pattern.len()` となるケースがある
- 非ループモード: 本来末尾を返すべきところ、切り捨て後のインデックスの値を返す

### 違反しているルール・原則

- 型安全性: 暗黙の切り捨てキャストが静かにバグを生む
- wasm32 対応: プロジェクトが明示的に wasm32 をターゲットにしている

### 該当箇所

- [src/circuit/generator.rs](../../src/circuit/generator.rs#L40-L47) `value_at` メソッド

### 解決策

`usize` を使わず `u64` のまま演算する。

```rust
pub fn value_at(&self, tick: u64) -> bool {
    let len = self.pattern.len() as u64;
    if self.is_loop {
        self.pattern[(tick % len) as usize]
    } else {
        self.pattern[tick.min(len - 1) as usize]
    }
}
```

### 影響範囲

- `src/circuit/generator.rs`: `value_at` メソッドのみ
- 実際に 2^32 tick 以上のシミュレーションは現実的ではないため、実用上の影響は極めて小さい

---

## ~~4. Pos に Display トレイトが未実装で手動フォーマットが分散している~~ (対応済み)

重要度: **low**

### 問題点

`Pos` 構造体に `Display` トレイトが実装されていないため、エラーメッセージで `(.0.x, .0.y)` のような手動フォーマットが `error.rs` 全バリアントに渡って分散している。

手動フォーマット箇所は以下の通りで、`src/base/error.rs` と `src/io/json.rs` の 2 ファイルに限定される。

### 該当箇所

**error.rs（人間向け表示、`"({x}, {y})"` 形式、8 箇所）:**
- [src/base/error.rs](../../src/base/error.rs#L4) `SelfLoop` — `.src.x, .src.y, .dst.x, .dst.y`
- [src/base/error.rs](../../src/base/error.rs#L7) `WireSrcNotFound` — `.0.x, .0.y`
- [src/base/error.rs](../../src/base/error.rs#L10) `WireDstNotFound` — `.0.x, .0.y`
- [src/base/error.rs](../../src/base/error.rs#L13) `DuplicateWire` — `.src.x, .src.y, .dst.x, .dst.y`
- [src/base/error.rs](../../src/base/error.rs#L16) `GeneratorTargetHasIncomingWires` — `.0.x, .0.y`
- [src/base/error.rs](../../src/base/error.rs#L19) `DuplicateGeneratorTarget` — `.0.x, .0.y`
- [src/base/error.rs](../../src/base/error.rs#L22) `EmptyGeneratorPattern` — `.0.x, .0.y`
- [src/base/error.rs](../../src/base/error.rs#L42) `UnknownCell` — `.0.x, .0.y`

**json.rs（JSON キー用、`"{x},{y}"` 形式、1 箇所）:**
- [src/io/json.rs](../../src/io/json.rs#L108) `format!("{},{}", pos.x, pos.y)`

### 違反しているルール・原則

- DRY 原則: 同じフォーマットロジックが複数箇所に分散
- 一貫性: フォーマット形式が箇所ごとに微妙に異なるリスクがある

### 解決策

2 つのフォーマットは用途が異なるため、それぞれ別の手段で解決する。

**error.rs 向け — `Pos` に `Display` を実装:**

```rust
impl std::fmt::Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
```

`error.rs` 側で `#[error("wire src does not exist in cells: {0}")]` のように簡略化できる。

**json.rs 向け — io モジュール内にフォーマット関数を切り出す:**

JSON キー形式 (`"0,0"`) は io モジュール固有の仕様であり、`Pos::Display` とは独立。io モジュール内に `fn pos_to_json_key(pos: &Pos) -> String` のようなヘルパーを定義し、フォーマットロジックを 1 箇所に集約する。

### 影響範囲

- `src/circuit/pos.rs`: `Display` 実装追加
- `src/base/error.rs`: `#[error(...)]` マクロ内のフォーマットを `{.src}` 等に簡略化
- `src/io/json.rs`: JSON キー生成をヘルパー関数に切り出し

---

## 総合所見

全体として、コードはモジュール分割が適切に行われており、テストカバレッジも十分。copilot-instructions.md のルール（モック禁止、テストでの実ファイル作成禁止等）に対する重大な違反は見当たらない。上記の指摘はいずれも保守性・命名の正確性に関するもので、既存の動作には影響しない。
