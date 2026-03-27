# JSON フォーマット仕様

回路 JSON ファイルにサブ回路の定義とインスタンス化を追加する。

## ルートオブジェクトの拡張

既存フィールドに加え、2 つのオプションフィールドを追加する:

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `wires` | `Wire[]` | はい | ワイヤ定義の配列（既存） |
| `input` | `Input[]` | いいえ | Input コンポーネントの配列（既存） |
| `output` | `Output[]` | いいえ | Output コンポーネントの配列（既存） |
| **`modules`** | `Module[]` | いいえ | モジュールインスタンスの配列（**新規**） |
| **`sub_circuits`** | `{ name: SubCircuit }` | いいえ | サブ回路定義のマップ（**新規**） |

## Module オブジェクト

モジュールインスタンスは、サブ回路定義を親回路に配置する。

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `type` | `string` | はい | モジュール種別。サブ回路の場合は `"sub"` |
| `sub_circuit` | `string` | `type` が `"sub"` のとき必須 | 使用するサブ回路定義の名前 |
| `input` | `[i32, i32][]` | はい | 親座標系での入力セル位置。要素数はサブ回路の `sub_input` と一致 |
| `output` | `[i32, i32][]` | はい | 親座標系での出力セル位置。要素数はサブ回路の `sub_output` と一致 |

`type` フィールドは将来の組み込みモジュール型（例: メモリ、クロック等）との混在を想定し、サブ回路参照名とは分離する。

## SubCircuit オブジェクト

サブ回路定義は再利用可能な回路テンプレートであり、ローカル座標系を持つ。

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `wires` | `Wire[]` | はい | ワイヤ定義の配列 |
| `sub_input` | `[i32, i32][]` | はい | インターフェース入力セル（ローカル座標） |
| `sub_output` | `[i32, i32][]` | はい | インターフェース出力セル（ローカル座標） |
| `modules` | `Module[]` | いいえ | ネストされたモジュールインスタンス |

サブ回路定義には `input`/`output` コンポーネント（Generator、Tester 等）は含められない。入力はモジュールインスタンス経由で親回路から供給される。

## サブ回路定義のスコープ

初期バージョンでは、全てのサブ回路定義はルートレベルの `sub_circuits` に配置する。サブ回路内の `modules` はルートレベルの定義のみ参照できる。

```
ルート
├── sub_circuits
│   ├── half_adder    ← 定義
│   └── full_adder    ← 定義（modules 内で half_adder を参照可能）
└── modules
    └── { type: "sub", sub_circuit: "full_adder" }  ← インスタンス
```

## 完全な例: 半加算器をサブ回路として使用

親回路は (0,0) から 2 本の Negative ワイヤで (1,0) と (1,1) に接続し、半加算器サブ回路で処理した結果を (2,0) と (2,1) に出力する。

```json
{
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "negative" },
    { "src": [0, 0], "dst": [1, 1], "kind": "negative" }
  ],
  "modules": [
    {
      "type": "sub",
      "sub_circuit": "half_adder",
      "input": [ [1, 0], [1, 1] ],
      "output": [ [2, 0], [2, 1] ]
    }
  ],
  "sub_circuits": {
    "half_adder": {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
        { "src": [0, 1], "dst": [1, 0], "kind": "positive" },
        { "src": [0, 0], "dst": [1, 1], "kind": "negative" },
        { "src": [0, 1], "dst": [1, 1], "kind": "negative" },
        { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
        { "src": [1, 1], "dst": [2, 0], "kind": "negative" },
        { "src": [2, 0], "dst": [3, 0], "kind": "negative" },
        { "src": [0, 0], "dst": [2, 1], "kind": "negative" },
        { "src": [0, 1], "dst": [2, 1], "kind": "negative" },
        { "src": [2, 1], "dst": [3, 1], "kind": "negative" }
      ],
      "sub_input": [ [0, 0], [0, 1] ],
      "sub_output": [ [3, 0], [3, 1] ]
    }
  }
}
```

## 全加算器の例: サブ回路の中でサブ回路を参照

全加算器は半加算器 2 つと OR ゲートで構成される。`full_adder` は `half_adder` を参照する。

```json
{
  "wires": [],
  "modules": [
    {
      "type": "sub",
      "sub_circuit": "full_adder",
      "input": [ [0, 0], [0, 1], [0, 2] ],
      "output": [ [5, 0], [5, 1] ]
    }
  ],
  "sub_circuits": {
    "half_adder": {
      "wires": [
        { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
        { "src": [0, 1], "dst": [1, 0], "kind": "positive" },
        { "src": [0, 0], "dst": [1, 1], "kind": "negative" },
        { "src": [0, 1], "dst": [1, 1], "kind": "negative" },
        { "src": [1, 0], "dst": [2, 0], "kind": "negative" },
        { "src": [1, 1], "dst": [2, 0], "kind": "negative" },
        { "src": [2, 0], "dst": [3, 0], "kind": "negative" },
        { "src": [0, 0], "dst": [2, 1], "kind": "negative" },
        { "src": [0, 1], "dst": [2, 1], "kind": "negative" },
        { "src": [2, 1], "dst": [3, 1], "kind": "negative" }
      ],
      "sub_input": [ [0, 0], [0, 1] ],
      "sub_output": [ [3, 0], [3, 1] ]
    },
    "full_adder": {
      "wires": [
        { "src": [3, 0], "dst": [4, 0], "kind": "positive" },
        { "src": [0, 2], "dst": [4, 1], "kind": "positive" },
        { "src": [3, 1], "dst": [8, 1], "kind": "positive" },
        { "src": [7, 1], "dst": [8, 1], "kind": "positive" }
      ],
      "sub_input": [ [0, 0], [0, 1], [0, 2] ],
      "sub_output": [ [8, 0], [8, 1] ],
      "modules": [
        {
          "type": "sub",
          "sub_circuit": "half_adder",
          "input": [ [0, 0], [0, 1] ],
          "output": [ [3, 0], [3, 1] ]
        },
        {
          "type": "sub",
          "sub_circuit": "half_adder",
          "input": [ [4, 0], [4, 1] ],
          "output": [ [7, 0], [7, 1] ]
        }
      ]
    }
  }
}
```

## 制約

### ポート列制約（Column Port Constraint）

入出力ポートの座標に以下の制約を設ける。これはモジュールインスタンスの `input`/`output` と、サブ回路定義の `sub_input`/`sub_output` の両方に適用される:

1. **同一 x 座標**: 全入力ポートは同一の x 座標を持つこと。全出力ポートも同一の x 座標を持つこと
2. **連続する y 座標**: 各グループ内で y 座標は連続する整数であること（y_base, y_base+1, y_base+2, ...）
3. **出力 > 入力**: 出力ポートの x 座標は入力ポートの x 座標より大きいこと

```
入力ポート列 (x=1)    出力ポート列 (x=5)
  (1, 0) ─ input[0]     (5, 0) ─ output[0]
  (1, 1) ─ input[1]     (5, 1) ─ output[1]
  (1, 2) ─ input[2]
```

この制約により、親回路の辞書順 (x, y) 処理において入力列が全て処理された後に出力列に到達することが保証され、階層的シミュレーションとフラット展開の等価性を維持する。

### モジュールインスタンス

4. `modules[i].input` の要素数は、参照するサブ回路の `sub_input` の要素数と一致すること
5. `modules[i].output` の要素数は、参照するサブ回路の `sub_output` の要素数と一致すること
6. `modules[i].output` の各座標は、親回路内でワイヤの `dst` になってはならない（入力ワイヤ禁止）
7. 異なるモジュール間で `output` 座標が重複してはならない
8. `modules[i].sub_circuit` で指定されたサブ回路定義が `sub_circuits` 内に存在すること

### サブ回路定義

9. `sub_input` の各座標はサブ回路内でワイヤの `dst` になってはならない（入力ワイヤ禁止）
10. サブ回路の依存グラフに循環があってはならない（A が B を参照し、B が A を参照するなど）

### 許容される組合せ

- モジュールの入力セルを複数モジュール間で共有（同じセル値を複数サブ回路に供給）
- モジュールの出力セルに Tester（Output コンポーネント）を設定
- モジュールの入力セルに Generator（Input コンポーネント）を設定
- モジュールの入力セルから他の親セルへのワイヤ
