# テストケース一覧

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景

追加するテストケースを 4 カテゴリに分類し、各テストの回路構成・検証内容を定義する。

## テスト一覧サマリ

| カテゴリ | テスト名 | 型 | 概要 | generator 必要 |
|---|---|---|---|---|
| 機能 | `eight_directions` | simulation | 8 方向ワイヤの即時/遅延伝搬 | No |
| 機能 | `feedback_oscillator` | simulation | 3 セル NOT リングの周期振動 | No |
| 機能 | `mixed_polarity_fan_in` | simulation | 正負混合ワイヤの合成 | No |
| 機能 | `isolated_cell_retains` | simulation | 入力なしセルの値保持 | No |
| 境界 | `i32_extreme_coords` | simulation | i32 極値付近の座標 | No |
| 境界 | `negative_coordinates` | simulation | 全負座標の回路 | No |
| 失敗 | `self_loop` | validation | self-loop ワイヤの拒否 | — |
| 失敗 | `duplicate_wire_same_kind` | validation | 同種多重辺の拒否 | — |
| 失敗 | `duplicate_wire_diff_kind` | validation | 異種多重辺の拒否 | — |
| 失敗 | `unknown_wire_kind` | validation | 不正な kind 文字列の拒否 | — |
| 総合 | `sr_latch` | simulation | NOR SR ラッチの set/hold | Yes |
| 総合 | `jk_flipflop` | simulation | JK-FF の set/reset/toggle | Yes |
| 総合 | `two_bit_counter` | simulation | 2 ビットカウンタ | Yes |
| 総合 | `full_adder` | simulation | 全加算器 | No |

## manifest 追記内容

```yaml
tests:
  # 既存
  - name: half_adder
    type: simulation
    path: simulation/half_adder
    comment: "半加算器の真理値表テスト"

  # === 機能テスト ===
  - name: eight_directions
    type: simulation
    path: simulation/eight_directions
    comment: "8方向ワイヤの即時伝搬・遅延伝搬の確認"

  - name: feedback_oscillator
    type: simulation
    path: simulation/feedback_oscillator
    comment: "3セルNOTリングの周期2振動"

  - name: mixed_polarity_fan_in
    type: simulation
    path: simulation/mixed_polarity_fan_in
    comment: "正負混合ワイヤの OR 合成"

  - name: isolated_cell_retains
    type: simulation
    path: simulation/isolated_cell_retains
    comment: "入力なしセルの値保持"

  # === 境界テスト ===
  - name: i32_extreme_coords
    type: simulation
    path: simulation/i32_extreme_coords
    comment: "i32 極値付近の座標での動作確認"

  - name: negative_coordinates
    type: simulation
    path: simulation/negative_coordinates
    comment: "全負座標の回路"

  # === 失敗テスト ===
  - name: self_loop
    type: validation
    path: validation/self_loop
    comment: "self-loop ワイヤを拒否"

  - name: duplicate_wire_same_kind
    type: validation
    path: validation/duplicate_wire_same_kind
    comment: "同種多重辺を拒否"

  - name: duplicate_wire_diff_kind
    type: validation
    path: validation/duplicate_wire_diff_kind
    comment: "異種多重辺を拒否"

  - name: unknown_wire_kind
    type: validation
    path: validation/unknown_wire_kind
    comment: "不正な kind 文字列を拒否"

  # === 総合テスト ===
  - name: sr_latch
    type: simulation
    path: simulation/sr_latch
    comment: "NOR SR ラッチの set/hold 動作"

  - name: jk_flipflop
    type: simulation
    path: simulation/jk_flipflop
    comment: "JK フリップフロップの動作確認"

  - name: two_bit_counter
    type: simulation
    path: simulation/two_bit_counter
    comment: "2ビットカウンタ"

  - name: full_adder
    type: simulation
    path: simulation/full_adder
    comment: "全加算器の真理値表テスト"
```

---

## 機能テスト

### eight_directions

**目的**: 8 方向全てのワイヤ配置で、即時伝搬（forward）と遅延伝搬（backward）が正しく動作することを確認する。

**回路**: 中心セル (5,5) から 8 方向にワイヤを配置。ソースセル (0,0) から (5,5) へ正極性ワイヤ。

```
(0,0) ──Pos──→ (5,5) ──Pos──→ (6,5)  右       (forward)
                     ──Pos──→ (5,6)  下       (forward)
                     ──Pos──→ (6,6)  右下     (forward)
                     ──Pos──→ (6,4)  右上     (forward)
                     ──Pos──→ (4,5)  左       (backward)
                     ──Pos──→ (5,4)  上       (backward)
                     ──Pos──→ (4,4)  左上     (backward)
                     ──Pos──→ (4,6)  左下     (backward)
```

セル処理順序 (x,y 辞書順):
```
(0,0) < (4,4) < (4,5) < (4,6) < (5,4) < (5,5) < (5,6) < (6,4) < (6,5) < (6,6)
```

- forward (dst > src): (5,6), (6,4), (6,5), (6,6) — 即時伝搬、tick 1 で反映
- backward (dst < src): (4,4), (4,5), (4,6), (5,4) — 遅延伝搬、tick 2 で反映

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [5, 5], "kind": "positive" },
    { "src": [5, 5], "dst": [6, 5], "kind": "positive" },
    { "src": [5, 5], "dst": [5, 6], "kind": "positive" },
    { "src": [5, 5], "dst": [6, 6], "kind": "positive" },
    { "src": [5, 5], "dst": [6, 4], "kind": "positive" },
    { "src": [5, 5], "dst": [4, 5], "kind": "positive" },
    { "src": [5, 5], "dst": [5, 4], "kind": "positive" },
    { "src": [5, 5], "dst": [4, 4], "kind": "positive" },
    { "src": [5, 5], "dst": [4, 6], "kind": "positive" }
  ]
}
```

**check.json** (tick 1 — forward のみ反映):
```json
{
  "ticks": 1,
  "cases": [
    {
      "name": "forward_immediate",
      "initial": { "0,0": true },
      "expected": {
        "5,5": true,
        "5,6": true, "6,4": true, "6,5": true, "6,6": true,
        "4,4": false, "4,5": false, "4,6": false, "5,4": false
      }
    },
    {
      "name": "forward_immediate_negative",
      "comment": "ソースが false の場合、全て false のまま",
      "initial": {},
      "expected": {
        "5,5": false,
        "5,6": false, "6,4": false, "6,5": false, "6,6": false,
        "4,4": false, "4,5": false, "4,6": false, "5,4": false
      }
    }
  ]
}
```

**注**: backward の遅延伝搬確認は per-case ticks が必要。ticks=2 のケースを追加:
```json
{
  "name": "backward_delayed",
  "ticks": 2,
  "initial": { "0,0": true },
  "expected": {
    "4,4": true, "4,5": true, "4,6": true, "5,4": true,
    "5,5": true, "5,6": true, "6,4": true, "6,5": true, "6,6": true
  }
}
```

### feedback_oscillator

**目的**: フィードバック（逆行ワイヤ）を含む回路で、遅延による周期的振動が正しく発生することを確認する。

**回路**: 3 セル NOT リング。全ワイヤ Negative。

```
(0,0) ──Neg──→ (1,0)  forward
(1,0) ──Neg──→ (2,0)  forward
(2,0) ──Neg──→ (0,0)  backward (delayed)
```

**動作トレース** (初期値: 全 false):

| tick | (0,0) | (1,0) | (2,0) | 備考 |
|------|-------|-------|-------|------|
| 0 | false | false | false | 初期 |
| 1 | true | false | true | (0,0): delayed NOT(false)=true |
| 2 | false | true | false | (0,0): delayed NOT(true)=false |
| 3 | true | false | true | tick 1 と同じ |

周期 2 で振動する。

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "negative" },
    { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
    { "src": [2, 0], "dst": [0, 0], "kind": "negative" }
  ]
}
```

**check.json**:
```json
{
  "ticks": 1,
  "cases": [
    {
      "name": "tick1",
      "expected": { "0,0": true, "1,0": false, "2,0": true }
    },
    {
      "name": "tick2",
      "ticks": 2,
      "expected": { "0,0": false, "1,0": true, "2,0": false }
    },
    {
      "name": "tick3_equals_tick1",
      "ticks": 3,
      "expected": { "0,0": true, "1,0": false, "2,0": true }
    }
  ]
}
```

### mixed_polarity_fan_in

**目的**: 正極性と負極性のワイヤが同一セルに入力される場合の OR 合成を確認する。

**回路**:
```
(0,0) ──Pos──→ (2,0)   a そのまま
(1,0) ──Neg──→ (2,0)   NOT(b)
```
out = a OR NOT(b)

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [2, 0], "kind": "positive" },
    { "src": [1, 0], "dst": [2, 0], "kind": "negative" }
  ]
}
```

**check.json**:
```json
{
  "ticks": 1,
  "cases": [
    { "name": "0_0", "initial": {}, "expected": { "2,0": true } },
    { "name": "0_1", "initial": { "1,0": true }, "expected": { "2,0": false } },
    { "name": "1_0", "initial": { "0,0": true }, "expected": { "2,0": true } },
    { "name": "1_1", "initial": { "0,0": true, "1,0": true }, "expected": { "2,0": true } }
  ]
}
```

### isolated_cell_retains

**目的**: 入力ワイヤを持たないセルが、tick をまたいでも値を保持し続けることを確認する。

**回路**: 2 セル。(0,0) は孤立、(1,0) は (0,0) からの出力先。

```
(0,0) ──Pos──→ (1,0)
```

(0,0) への入力ワイヤはないため、initial で設定した値が保持される。

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
  ]
}
```

**check.json**:
```json
{
  "ticks": 1,
  "cases": [
    {
      "name": "retains_true",
      "ticks": 3,
      "initial": { "0,0": true },
      "expected": { "0,0": true, "1,0": true }
    },
    {
      "name": "retains_false",
      "ticks": 3,
      "initial": {},
      "expected": { "0,0": false, "1,0": false }
    }
  ]
}
```

---

## 境界テスト

### i32_extreme_coords

**目的**: i32 の極値付近の座標で回路が正しく構築・シミュレーションできることを確認する。

**回路**: i32::MAX 付近と i32::MIN 付近の座標を使用。

```
(2147483646, 0) ──Neg──→ (2147483647, 0)
(-2147483648, 0) ──Neg──→ (-2147483647, 0)
```

2 つの独立した NOT ゲート。一方は i32::MAX 付近、もう一方は i32::MIN 付近。

**circuit.json**:
```json
{
  "wires": [
    { "src": [2147483646, 0], "dst": [2147483647, 0], "kind": "negative" },
    { "src": [-2147483648, 0], "dst": [-2147483647, 0], "kind": "negative" }
  ]
}
```

**check.json**:
```json
{
  "ticks": 1,
  "cases": [
    {
      "name": "all_false_input",
      "expected": {
        "2147483647,0": true,
        "-2147483647,0": true
      }
    },
    {
      "name": "max_set",
      "initial": { "2147483646,0": true },
      "expected": {
        "2147483647,0": false,
        "-2147483647,0": true
      }
    }
  ]
}
```

### negative_coordinates

**目的**: 全座標が負のみの回路が正しく動作することを確認する。

**回路**: 半加算器と同等の XOR 回路を全負座標で構成。

```
(-4,0) a   (-3,0) or    (-2,0) nand_xor   (-1,0) sum
(-4,1) b   (-3,1) nand
```

**circuit.json**:
```json
{
  "wires": [
    { "src": [-4, 0], "dst": [-3, 0], "kind": "positive" },
    { "src": [-4, 1], "dst": [-3, 0], "kind": "positive" },
    { "src": [-4, 0], "dst": [-3, 1], "kind": "negative" },
    { "src": [-4, 1], "dst": [-3, 1], "kind": "negative" },
    { "src": [-3, 0], "dst": [-2, 0], "kind": "negative" },
    { "src": [-3, 1], "dst": [-2, 0], "kind": "negative" },
    { "src": [-2, 0], "dst": [-1, 0], "kind": "negative" }
  ]
}
```

**check.json**: XOR 真理値表と同一。

```json
{
  "ticks": 1,
  "cases": [
    { "name": "0_xor_0", "expected": { "-1,0": false } },
    { "name": "0_xor_1", "initial": { "-4,1": true }, "expected": { "-1,0": true } },
    { "name": "1_xor_0", "initial": { "-4,0": true }, "expected": { "-1,0": true } },
    { "name": "1_xor_1", "initial": { "-4,0": true, "-4,1": true }, "expected": { "-1,0": false } }
  ]
}
```

---

## 失敗テスト

### self_loop

**目的**: self-loop ワイヤが拒否されることを確認する。

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [0, 0], "kind": "positive" }
  ]
}
```

**expected.json**:
```json
{
  "error_contains": "self-loop wire is not allowed"
}
```

### duplicate_wire_same_kind

**目的**: 同一 (src, dst) ペアで同種の重複ワイヤが拒否されることを確認する。

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" }
  ]
}
```

**expected.json**:
```json
{
  "error_contains": "duplicate wire is not allowed"
}
```

### duplicate_wire_diff_kind

**目的**: 同一 (src, dst) ペアで異種の重複ワイヤが拒否されることを確認する。

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
    { "src": [0, 0], "dst": [1, 0], "kind": "negative" }
  ]
}
```

**expected.json**:
```json
{
  "error_contains": "duplicate wire is not allowed"
}
```

### unknown_wire_kind

**目的**: 不正な kind 文字列が拒否されることを確認する。

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "invalid" }
  ]
}
```

**expected.json**:
```json
{
  "error_contains": "wire kind must be positive or negative"
}
```

---

## 総合テスト

### full_adder

**目的**: 全加算器（carry-in 付き）の真理値表を検証する。組合せ回路。

**回路構成**: 2 つの半加算器 + OR ゲートで構成。

```
入力: a=(0,0), b=(0,1), cin=(0,2)

半加算器 1: XOR(a,b)=partial_sum, AND(a,b)=partial_carry
半加算器 2: XOR(partial_sum, cin)=sum, AND(partial_sum, cin)=carry2
carry = OR(partial_carry, carry2)

セル配置:
  (0,0) a      (1,0) or1     (2,0) nand_xor1  (3,0) partial_sum   (4,0) or2     (5,0) nand_xor2  (6,0) sum
  (0,1) b      (1,1) nand1   (2,1) nand_ab     (3,1) partial_carry (4,1) nand2   (5,1) nand_sc    (6,1) carry2   (7,0) carry
  (0,2) cin                                                                                        (7,1) or_carry
```

ワイヤ数が多いため、実装時に詳細を確定する。8 ケース (2^3 入力) の真理値表テスト。

**check.json (概要)**:
```json
{
  "ticks": 1,
  "cases": [
    { "name": "0_0_0", "expected": { "sum": false, "carry": false } },
    { "name": "0_0_1", "initial": { "0,2": true }, "expected": { "sum": true, "carry": false } },
    { "name": "0_1_0", "initial": { "0,1": true }, "expected": { "sum": true, "carry": false } },
    { "name": "0_1_1", "initial": { "0,1": true, "0,2": true }, "expected": { "sum": false, "carry": true } },
    { "name": "1_0_0", "initial": { "0,0": true }, "expected": { "sum": true, "carry": false } },
    { "name": "1_0_1", "initial": { "0,0": true, "0,2": true }, "expected": { "sum": false, "carry": true } },
    { "name": "1_1_0", "initial": { "0,0": true, "0,1": true }, "expected": { "sum": false, "carry": true } },
    { "name": "1_1_1", "initial": { "0,0": true, "0,1": true, "0,2": true }, "expected": { "sum": true, "carry": true } }
  ]
}
```

### sr_latch

**目的**: NOR ベースの SR ラッチが set/hold/reset を正しく動作することを確認する。ジェネレーターを使用。

**回路構成**:

```
R=(0,0)   →(1,0) OR_Q  →(2,0) Q     (NOR: NOT(OR(R, Q̄)))
S=(0,1)   →(1,1) OR_Q̄  →(2,1) Q̄    (NOR: NOT(OR(S, Q)))

フィードバック:
(2,1) Q̄ ──Pos──→ (1,0) OR_Q   (backward, delayed)
(2,0) Q  ──Pos──→ (1,1) OR_Q̄  (backward, delayed)
```

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
    { "src": [0, 1], "dst": [1, 1], "kind": "positive" },
    { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
    { "src": [1, 1], "dst": [2, 1], "kind": "negative" },
    { "src": [2, 1], "dst": [1, 0], "kind": "positive" },
    { "src": [2, 0], "dst": [1, 1], "kind": "positive" }
  ]
}
```

**check.json**: per-case generator で S/R を制御。

```json
{
  "ticks": 4,
  "cases": [
    {
      "name": "set_then_hold",
      "comment": "S=1 で set → S=0 で hold。Q=1 を維持",
      "generators": [
        { "target": [0, 0], "pattern": "0000" },
        { "target": [0, 1], "pattern": "1000" }
      ],
      "expected": { "2,0": true, "2,1": false }
    },
    {
      "name": "set_then_reset",
      "comment": "S=1 で set → R=1 で reset。Q=0",
      "generators": [
        { "target": [0, 0], "pattern": "001000" },
        { "target": [0, 1], "pattern": "100000" }
      ],
      "ticks": 6,
      "expected": { "2,0": false, "2,1": true }
    }
  ]
}
```

**注**: フィードバックの遅延により安定に 2 tick かかるため、各操作後に十分な tick を設ける。

### jk_flipflop

**目的**: NAND ベースの JK フリップフロップの set/reset/hold/toggle の動作を確認する。ジェネレーターを使用。

**回路構成**: 3 入力 NAND + NAND SR ラッチ。

```
J=(0,0), K=(0,1), CLK=(0,2)

S̄ = NAND(J, CLK, Q̄)  → (1,0)  3本のNegativeワイヤ
R̄ = NAND(K, CLK, Q)   → (1,1)  3本のNegativeワイヤ

Q  = NAND(S̄, Q̄)       → (2,0)  S̄とQ̄のNegativeワイヤ
Q̄  = NAND(R̄, Q)        → (2,1)  R̄とQのNegativeワイヤ

フィードバック:
Q̄(2,1) ──Neg──→ (1,0)  S̄への入力 (backward, delayed)
Q(2,0)  ──Neg──→ (1,1)  R̄への入力 (backward, delayed)
Q̄(2,1) ──Neg──→ (2,0)  Q への入力 (forward, immediate)
            ↑ 注: (2,1)→(2,0) は dst < src なので backward (delayed)
```

遅延方向の確認:
- (2,1) → (1,0): dst=(1,0) < src=(2,1) → delayed ✓
- (2,0) → (1,1): dst=(1,1) < src=(2,0) → delayed ✓
- (2,1) → (2,0): dst=(2,0) < src=(2,1) → delayed ✓

**circuit.json**:
```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "negative" },
    { "src": [0, 2], "dst": [1, 0], "kind": "negative" },
    { "src": [2, 1], "dst": [1, 0], "kind": "negative" },
    { "src": [0, 1], "dst": [1, 1], "kind": "negative" },
    { "src": [0, 2], "dst": [1, 1], "kind": "negative" },
    { "src": [2, 0], "dst": [1, 1], "kind": "negative" },
    { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
    { "src": [2, 1], "dst": [2, 0], "kind": "negative" },
    { "src": [1, 1], "dst": [2, 1], "kind": "negative" },
    { "src": [2, 0], "dst": [2, 1], "kind": "negative" }
  ]
}
```

**check.json**: per-case generator で J, K, CLK を制御。各操作後に安定するまでの tick を含める。

```json
{
  "ticks": 6,
  "cases": [
    {
      "name": "set",
      "comment": "J=1, K=0, CLK=1 → Q=1",
      "generators": [
        { "target": [0, 0], "pattern": "111000" },
        { "target": [0, 1], "pattern": "0" },
        { "target": [0, 2], "pattern": "111000" }
      ],
      "expected": { "2,0": true, "2,1": false }
    },
    {
      "name": "reset",
      "comment": "まず set してから、J=0, K=1, CLK=1 で reset → Q=0",
      "generators": [
        { "target": [0, 0], "pattern": "11100000" },
        { "target": [0, 1], "pattern": "00011100" },
        { "target": [0, 2], "pattern": "11111100" }
      ],
      "ticks": 8,
      "expected": { "2,0": false, "2,1": true }
    }
  ]
}
```

**注**: LGCELL2 のフィードバック遅延モデルでは、JK-FF の安定に数 tick かかる。実装時にトレースを検証し、適切な tick 数を確定する。

### two_bit_counter

**目的**: クロック信号を入力として、2 ビットのバイナリカウントを行うことを確認する。ジェネレーターを使用。

**回路構成**: T フリップフロップ 2 段のリプルカウンタ。

T-FF はフィードバック NOT ループで構成:
```
bit0: (0,0) ──Neg──→ (1,0)
      (1,0) ──Pos──→ (0,0) (backward, delayed)

bit1: (2,0) ──Neg──→ (3,0)
      (3,0) ──Pos──→ (2,0) (backward, delayed)
```

bit0 の出力変化を bit1 のトグル条件とする接続が必要。LGCELL2 のモデルでエッジ検出を実現するため、回路設計は実装時に詳細化する。

**注**: LGCELL2 のレベルトリガモデルではエッジ検出が本来的に難しい。tick ベースの遅延を活用した設計が必要であり、実装時に回路トレースで正確性を検証する。

---

## 既存テストの整理

以下の既存ユニットテストは manifest テストと重複する観点を持つが、**移動は推奨しない**。ユニットテストは Rust コードレベルでの動作保証、manifest テストはデータ駆動の統合テストとして別の役割を果たす。

| 既存ユニットテスト | 対応する manifest テスト |
|---|---|
| `circuit_rejects_self_loop` | `self_loop` (validation) |
| `circuit_rejects_duplicate_wire_*` | `duplicate_wire_*` (validation) |
| `nand_is_constructed_by_two_negative_wires` | (基本ゲートテスト追加を検討) |

基本ゲート（NOT, OR, NAND, AND, NOR）の真理値表テストを manifest に追加するかは、テスト数とメンテナンスコストを考慮して実装時に判断する。
