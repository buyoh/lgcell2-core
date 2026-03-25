# SR ラッチ set_then_reset テストケースの修正

SR ラッチの set → reset 動作テストが失敗している。ジェネレーターパターンのタイミング設計を修正する必要がある。

作成日: 2026-03-25
ステータス: 未着手

## 背景・動機

expand-test-manifest タスクで追加した `sr_latch` テストの `set_then_reset` ケースが失敗している。

### 失敗内容

```
assertion `left == right` failed: Mismatch at 2,0 in test case set_then_reset:
expected false, got true
```

Q=(2,0) が `true`（set 状態）のまま `false`（reset 状態）に遷移していない。

### 現在のジェネレーターパターン

```json
{
  "generators": [
    { "target": [0, 0], "pattern": "001000" },  // R: tick2 で ON
    { "target": [0, 1], "pattern": "100000" }   // S: tick0 で ON
  ],
  "ticks": 6
}
```

## 設計・方針

NOR SR ラッチのフィードバック遅延では、backward ワイヤ `(2,1)→(1,0)` と `(2,0)→(1,1)` が各 tick で 1 tick 遅延する。set 操作が安定するまでに複数 tick が必要であり、reset 信号の投入タイミングが早すぎる可能性がある。

### 調査項目

1. SR ラッチ回路のセル処理順を確認し、各 tick での状態遷移をトレースする
2. set 操作が安定するまでの必要 tick 数を特定する
3. reset 信号の適切な投入タイミングを決定する
4. ジェネレーターパターンを修正してテストを通す
