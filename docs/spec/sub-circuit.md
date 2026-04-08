# サブ回路仕様

サブ回路（Sub-Circuit）は、再利用可能な回路テンプレートを定義し、親回路内で複数回インスタンス化する仕組みです。

## 概念

### サブ回路定義（`subs`）

再利用可能な回路テンプレートです。独立したローカル座標系を持ち、入出力インターフェースを宣言します。

- `wires`: サブ回路内部のワイヤ接続
- `sub_input`: 外部から値を受け取る入力ポート座標
- `sub_output`: 外部へ値を返す出力ポート座標
- `modules`: ネストされたサブ回路モジュール（別のサブ回路を内部で使用）

### モジュールインスタンス（`modules`）

サブ回路定義を親回路に配置したものです。親回路のどのセルを入力・出力として使うかを指定します。

- `type`: 現在は `"sub"` のみ
- `sub_circuit`: 参照するサブ回路名
- `input`: 親回路側の入力ポート座標
- `output`: 親回路側の出力ポート座標

## JSON フォーマット

### ルートオブジェクトの拡張フィールド

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `modules` | `Module[]` | いいえ | モジュールインスタンスの配列 |
| `subs` | `{ [name: string]: SubCircuit }` | いいえ | サブ回路定義のマップ |

### Module オブジェクト

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `type` | `string` | はい | `"sub"` |
| `sub_circuit` | `string` | はい | `subs` 内のキー名 |
| `input` | `[i32, i32][]` | はい | 親回路側の入力ポート座標 |
| `output` | `[i32, i32][]` | はい | 親回路側の出力ポート座標 |

### SubCircuit オブジェクト

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `wires` | `Wire[]` | はい | 内部ワイヤ |
| `sub_input` | `[i32, i32][]` | はい | 入力ポート座標 |
| `sub_output` | `[i32, i32][]` | はい | 出力ポート座標 |
| `modules` | `Module[]` | いいえ | ネストモジュール |

## 制約

### バリデーションエラー

| 制約 | エラー |
|---|---|
| `sub_circuit` が `subs` に存在しない | `sub-circuit not found` |
| `subs` 間の参照が循環する | `circular dependency detected` |
| `modules[].input` の要素数 ≠ `sub_input` の要素数 | `sub_input count mismatch` |
| `modules[].output` の要素数 ≠ `sub_output` の要素数 | `sub_output count mismatch` |
| `sub_input` に入力ワイヤが接続されている | `sub_input must not have incoming wires` |
| モジュール出力セルに入力ワイヤが接続されている | `module output must not have incoming wires` |
| モジュール出力セルが複数モジュール間で重複 | `duplicate module output` |

### ポート列制約（Column Port Constraint）

入出力ポートは以下の配置規則を満たす必要があります。

- **同一 x 座標**: ポート列内のすべてのポートは同じ x を持つ
- **連続 y 座標**: ポート列内の y は隙間なく連続する
- **出力列 > 入力列**: 出力ポートの x は入力ポートの x より大きい

この制約はモジュールインスタンスの `input`/`output` とサブ回路定義の `sub_input`/`sub_output` の両方に適用されます。

```
親回路の処理順序:
  ... → [入力ポート列] → [出力ポート列] → ...
          ↓                    ↑
         サブ回路内部で伝搬
```

## シミュレーション動作

### 階層的シミュレーション

各モジュールインスタンスは内部に独立したシミュレータを保持します。

1. 親回路の tick 処理中、モジュールの最初の出力セルに到達した時点でサブ回路を評価
2. 親の入力ポートの値をサブ回路の `sub_input` に注入
3. サブ回路を 1 tick 実行
4. サブ回路の `sub_output` の値を親の出力ポートに反映

### 状態の独立性

- 各モジュールインスタンスは独立した内部状態を持つ（同じサブ回路を複数回使っても状態は共有しない）
- サブ回路内部の状態は tick 間で保持される（順序回路サブ回路に対応）
- サブ回路内部のセルは親回路の `getState()` / `getCell()` には含まれない

### View モード

View モード（`--view`）はサブ回路を含む回路に未対応です。サブ回路を含む回路を View モードで開くとエラーになります。通常モード（JSON 出力）では問題なく動作します。

## 使用例

### 基本: インバータモジュール

```json
{
  "wires": [],
  "modules": [
    {
      "type": "sub",
      "sub_circuit": "inverter",
      "input": [[0, 0]],
      "output": [[1, 0]]
    }
  ],
  "subs": {
    "inverter": {
      "wires": [{ "src": [0, 0], "dst": [1, 0], "kind": "negative" }],
      "sub_input": [[0, 0]],
      "sub_output": [[1, 0]]
    }
  }
}
```

セル `(0,0)` の値を反転して `(1,0)` に出力します。

### 同一サブ回路の複数インスタンス

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
    { "src": [0, 0], "dst": [3, 0], "kind": "positive" }
  ],
  "modules": [
    {
      "type": "sub",
      "sub_circuit": "inverter",
      "input": [[1, 0]],
      "output": [[2, 0]]
    },
    {
      "type": "sub",
      "sub_circuit": "inverter",
      "input": [[3, 0]],
      "output": [[4, 0]]
    }
  ],
  "subs": {
    "inverter": {
      "wires": [{ "src": [0, 0], "dst": [1, 0], "kind": "negative" }],
      "sub_input": [[0, 0]],
      "sub_output": [[1, 0]]
    }
  }
}
```

同じ `inverter` を 2 回インスタンス化。各インスタンスは独立した状態を持ちます。

### ネストされたサブ回路

サブ回路の中で別のサブ回路を使うことができます。

```json
{
  "wires": [],
  "modules": [
    {
      "type": "sub",
      "sub_circuit": "double_inverter",
      "input": [[0, 0]],
      "output": [[1, 0]]
    }
  ],
  "subs": {
    "inverter": {
      "wires": [{ "src": [0, 0], "dst": [1, 0], "kind": "negative" }],
      "sub_input": [[0, 0]],
      "sub_output": [[1, 0]]
    },
    "double_inverter": {
      "wires": [],
      "sub_input": [[0, 0]],
      "sub_output": [[3, 0]],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "inverter",
          "input": [[0, 0]],
          "output": [[1, 0]]
        },
        {
          "type": "sub",
          "sub_circuit": "inverter",
          "input": [[1, 0]],
          "output": [[3, 0]]
        }
      ]
    }
  }
}
```

`double_inverter` は内部で `inverter` を 2 回使い、NOT(NOT(x)) = x（バッファ）として動作します。

### 複数サブ回路が共通の依存先を参照

```json
{
  "wires": [],
  "modules": [
    { "type": "sub", "sub_circuit": "a_inv", "input": [[0, 0]], "output": [[1, 0]] },
    { "type": "sub", "sub_circuit": "b_buf", "input": [[2, 0]], "output": [[3, 0]] }
  ],
  "subs": {
    "c_not": {
      "wires": [{ "src": [0, 0], "dst": [1, 0], "kind": "negative" }],
      "sub_input": [[0, 0]],
      "sub_output": [[1, 0]]
    },
    "a_inv": {
      "wires": [],
      "sub_input": [[0, 0]],
      "sub_output": [[1, 0]],
      "modules": [
        { "type": "sub", "sub_circuit": "c_not", "input": [[0, 0]], "output": [[1, 0]] }
      ]
    },
    "b_buf": {
      "wires": [],
      "sub_input": [[0, 0]],
      "sub_output": [[3, 0]],
      "modules": [
        { "type": "sub", "sub_circuit": "c_not", "input": [[0, 0]], "output": [[1, 0]] },
        { "type": "sub", "sub_circuit": "c_not", "input": [[1, 0]], "output": [[3, 0]] }
      ]
    }
  }
}
```

`a_inv`（1 回 NOT = 反転）と `b_buf`（2 回 NOT = バッファ）が共通の `c_not` を参照します。依存関係が DAG であれば、複数のサブ回路が同じ定義を共有できます。

## WASM API

WASM API では、JSON パス（`WasmSimulator.fromJson()`）と型付きパス（`new WasmSimulator()`）の両方でサブ回路を利用できます。

### 型付きパスの型定義

```typescript
interface WasmCircuitInput {
    wires: WasmWireInput[];
    generators?: WasmGeneratorInput[];
    modules?: WasmModuleInput[];
    sub_circuits?: Map<string, WasmSubCircuitInput>;
}

interface WasmModuleInput {
    type: string;
    sub_circuit: string | undefined;
    input: [number, number][];
    output: [number, number][];
}

interface WasmSubCircuitInput {
    wires: WasmWireInput[];
    sub_input: [number, number][];
    sub_output: [number, number][];
    modules?: WasmModuleInput[];
}
```

### API の動作

| メソッド | サブ回路の影響 |
|---|---|
| `run()` / `runSteps()` | サブ回路を含む場合も正常に動作 |
| `getState()` / `getCell()` | 親回路のセルのみ返す（サブ回路内部は不可視） |
| `setCell()` | 親回路のセルのみ設定可能 |
| `currentTick` | 親回路の tick カウント |
