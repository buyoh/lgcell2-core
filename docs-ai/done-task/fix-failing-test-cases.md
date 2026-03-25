# 失敗テストケースの修正

expand-test-manifest で追加したテストのうち、3件が失敗または除外されている。回路設計・ジェネレーターパターンを修正してテストを通す。

作成日: 2026-03-25
ステータス: 完了

## 背景・動機

expand-test-manifest タスクで追加したテストのうち、以下の 3 件が失敗または除外されている。

| テスト | 状態 | 問題 |
|---|---|---|
| `sr_latch` / `set_then_reset` | FAIL | reset タイミング不足で Q が true のまま |
| `full_adder` | マニフェスト除外 | 回路設計が誤り（AND ゲート構成の見直しが必要） |
| `two_bit_counter` | マニフェスト除外 | bit1 トグル検証が失敗（エッジ検出設計の見直しが必要） |

すべてのテストデータは `resources/tests/simulation/` に残っており、マニフェスト上もコメントアウトで記載済み。

## 各テストの詳細

### 1. sr_latch / set_then_reset（マニフェスト登録済み・テスト FAIL）

**失敗内容**:
```
Mismatch at 2,0 in test case set_then_reset: expected false, got true
```

Q=(2,0) が set 状態のまま reset に遷移していない。

**現在のジェネレーターパターン**:
```json
{
  "generators": [
    { "target": [0, 0], "pattern": "001000" },
    { "target": [0, 1], "pattern": "100000" }
  ],
  "ticks": 6
}
```

**原因**: NOR SR ラッチのフィードバック遅延（backward ワイヤ `(2,1)→(1,0)`, `(2,0)→(1,1)`）により、set 操作の安定化に複数 tick が必要。reset 信号の投入タイミング（tick 2）が早すぎる可能性がある。

**修正方針**:
1. SR ラッチ回路のセル処理順を確認し、各 tick での状態遷移をトレースする
2. set 操作が安定するまでの必要 tick 数を特定する
3. reset 信号の適切な投入タイミングを決定し、ジェネレーターパターンを修正する

### 2. full_adder（マニフェスト除外・テストデータ残存）

**失敗内容**:
```
Mismatch at 3,2 in test case 0_0_0: expected false, got true
```

carry 出力セル (3,2) が入力 0,0,0 の場合でも true になっている。

**原因（前回の分析）**: 前回の調査では「LGCELL2 で AND ゲートが実装不可」と結論づけたが、この分析には誤りがある。LGCELL2 では NAND（= 複数 negative ワイヤ）と NOT（= 単体 negative ワイヤ）が実装可能であり、`AND(a,b) = NOT(NAND(a,b))` で AND ゲートを構成できる。問題は回路設計の誤りであり、適切な中間セルを追加すれば全加算器は実装可能。

**修正方針**:
1. 全加算器の回路を NAND/NOT/OR のみで再設計する
   - `AND(a,b) = NOT(NAND(a,b))`: NAND の出力セルから NOT ワイヤで中間セルへ
   - carry = `OR(AND(a,b), AND(partial_sum, cin))`
2. circuit.json を修正し、check.json の expected セル座標を更新する

### 3. two_bit_counter（マニフェスト除外・テストデータ残存）

**失敗内容**:
```
Mismatch at 2,0 in test case bit1_toggles_on_bit0_change: expected true, got false
```

**原因**: bit0 の NOT フィードバックループ `(0,0)→(1,0)→(0,0)` で bit0 がトグルするが、bit1 への接続 `(1,0)→(2,0)` が negative ワイヤであるため、bit0 の変化がそのまま NOT として伝わる。リプルカウンタのエッジ検出は LGCELL2 のレベルトリガモデルでは本来的に困難。

**修正方針**:
1. LGCELL2 の遅延モデルを活用した分周器の設計を検討する
2. 実現不可能な場合は、テストケースの expected 値を実際の動作に合わせるか、テストケース自体を別の回路（例：シフトレジスタ）に差し替える

## ステップ

1. ✅ sr_latch の状態遷移トレースを行い、set_then_reset のジェネレーターパターンを修正する
   - 原因: backward フィードバックワイヤの遅延により、R=1 が 1 tick では不十分。R を 2 tick 保持することで安定化
   - パターン変更: R="001000"→"0011000"（tick 2-3 で reset）、ticks=6→7
2. ✅ full_adder の回路を AND=NOT(NAND) で再設計し、circuit.json と check.json を修正する
   - 構成: OR→NAND→NOT(NAND)=AND、XOR=AND(OR,NAND)
   - 中間セル追加により全 forward ワイヤでの 1 tick 完結を実現（出力: (6,0)=sum, (6,1)=carry）
3. ✅ two_bit_counter → shift_register に差し替え
   - リプルカウンタは level-triggered モデルでは実現不可（bit0 と bit1 が同期して振動するだけ）
   - 代替: 2 段シフトレジスタ（backward ワイヤによる 1 tick 遅延 × 2 段）
4. ✅ 修正したテストをマニフェストに復帰（コメントアウト解除）し、全テスト PASS を確認する
   - 全 47 テスト PASS
