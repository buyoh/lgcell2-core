# 全加算器テスト実装の失敗分析

発生日: 2026-03-25

## 問題概要

expand-test-manifest で計画していた **full_adder** テストケースが LGCELL2 のセル計算モデルの制限により実装不可能であることが判明しました。

## 原因分析

### 1. LGCELL2 セルの計算ロジック

LGCELL2 の各セルへの信号入力は以下のロジックで処理されます：

```rust
let mut next_value = false;
for wire in incoming_wires {
    next_value = next_value || wire.propagate(src_value);
}
```

つまり、複数の incoming wire の伝搬値を **OR** で合成します。

### 2. Wire の極性

- **Positive wire**: `propagate(v) = v` （値をそのまま伝搬）
- **Negative wire**: `propagate(v) = !v` （値を反転して伝搬）

### 3. ロジックの帰結

| 入力パターン | セルへの接続形式 | 計算結果 |
|---|---|---|
| 複数 positive | `a OR b` | or ロジック |
| 複数 negative | `!(!(a) \| !(b))` = `a AND b` | and ロジック ❌ |
| mixed | `a \| !b` (正と反転の混合) | or ロジック |

**問題**: 複数の negative wire は NAND ロジック (= NOT(a AND b)) を実装するが、双 negative で AND ロジックを得ることは理論上可能でも、実装上は OR ロジックになります。

詳細なトレース例：
- (3,0) へ (0,0) と (0,1) からの negative ワイヤ接続
  - `next_value = false | !src(0,0) | !src(0,1)`
  - `= false | !a | !b`
  - `= NOT(a AND b)` = NAND ロジック

しかし実際には全加算器の AND(a,b) を実装する必要があります。

### 4. AND ゲートの実装不可性

全加算器の carry 計算：
```
carry = (a AND b) OR (XOR(a,b) AND cin)
```

LGCELL2 では以下のロジックのみ実装可能：
- OR: positive ワイヤ複数
- NAND: negative ワイヤ複数
- NOT: negative ワイヤ 1 本
- XOR: `NAND(OR(a,b), NAND(a,b))`

AND ゲートを実現するには、セルへの positive ワイヤのみ接続が必要ですが、これは複数 positive = OR ロジックになり、AND ではありません。

## 解決案の検討

### 案 1: 深い理解による設計変更

全加算器を OR/NAND/NOT のみで実装する booleano algebra 変換：
```
carry = OR(NAND(NOT(a), NOT(b)), NAND(NOT(XOR(a,b)), NOT(cin)))
```

これは理論的には可能ですが、セル数の増加とワイヤの複雑化で実装が困難です。

### 案 2: テストケースの遅延

LGCELL2 の基本ロジック拡張（AND 出力専用ターミナルの追加など）により、将来的に対応することを検討します。

### 案 3: 代替テストケース

より単純な組合せ回路（例：XOR、パリティジェネレータなど）を代替として設計します。

## 結論

**現在の LGCELL2 モデルでは AND ゲートが実装不可のため、全加算器テストは保留します。**

ドキュメント更新：
- expand-test-manifest/README.md に記載
- テストマニフェストから full_adder を削除

## 参考資料

- [セル計算ロジック実装](../../../../src/simulation/engine.rs#L85-L110)
