# 同一端点間の重複ワイヤ禁止

同一の `(src, dst)` ペアに対して複数のワイヤを配置できないよう制約を追加する。

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

現在、`Circuit::new()` は self-loop と端点の存在チェックのみを行い、同一 `(src, dst)` 間に複数のワイヤ（例: Positive と Negative の両方）を配置することを許容している。回路の意味的な曖昧さを排除するため、ある 2 点間に配置できるワイヤは高々 1 つとする制約を導入する。

## 設計・方針

### 制約の定義

- **有向ペア `(src, dst)` の一意性**: 同じ `(src, dst)` を持つワイヤは `WireKind` に関わらず 1 つだけ許可する
- 逆方向 `(dst, src)` は別のペアとして扱うため、A→B と B→A の共存は許可される

### 修正対象

#### 1. `src/circuit/circuit.rs` — `Circuit::new()`

バリデーションループ内で `(src, dst)` の重複チェックを追加する。`HashSet<(Pos, Pos)>` を用いて既出ペアを記録し、重複検出時にエラーを返す。

```rust
// 既存の検証ループの前または中に追加
let mut seen_pairs: HashSet<(Pos, Pos)> = HashSet::new();
for wire in &wires {
    // ... 既存の self-loop / endpoint チェック ...

    if !seen_pairs.insert((wire.src, wire.dst)) {
        return Err(format!(
            "duplicate wire is not allowed: src=({}, {}), dst=({}, {})",
            wire.src.x, wire.src.y, wire.dst.x, wire.dst.y
        ));
    }
}
```

#### 2. `src/circuit/circuit_tests.rs` — テスト追加

重複ワイヤが拒否されることを確認するテストケースを追加する。

- 同一 `(src, dst)` ・同一 `WireKind` のケース
- 同一 `(src, dst)` ・異なる `WireKind` のケース

#### 3. `src/io/json_tests.rs` — テスト追加

JSON 入力経由で重複ワイヤが拒否されることを確認するテストケースを追加する。

#### 4. `docs-ai/architecture/data-model.md` — ドキュメント更新

Wire セクションの制約リストと Circuit セクションの「構築時の検証」に重複ワイヤ禁止の記述を追加する。

具体的な修正箇所:
- Wire の「**制約:**」セクションに `(src, dst)` 一意性の記述を追加
- Circuit の「構築時の検証」のリストに項目 3 として追加
