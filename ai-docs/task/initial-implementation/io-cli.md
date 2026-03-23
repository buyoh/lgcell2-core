# JSON I/O & CLI 設計

回路定義の JSON フォーマットと CLI インターフェースを設計する。

作成日: 2026-03-23
ステータス: 設計完了（未実装）

## 背景・動機

配線情報を JSON で受け取り、シミュレーション結果を標準出力へ表示する CLI が必要。将来的に wasm からも同じ JSON パース・生成ロジックを利用する。

## 設計・方針

### 入力 JSON フォーマット

```json
{
  "cells": [
    { "x": 0, "y": 0, "initial": 0 },
    { "x": 1, "y": 0, "initial": 1 },
    { "x": 2, "y": 0, "initial": 0 }
  ],
  "wires": [
    { "src": [0, 0], "dst": [1, 0], "kind": "positive" },
    { "src": [1, 0], "dst": [2, 0], "kind": "negative" }
  ]
}
```

- `cells[].initial`: `0` または `1`（通常モード）。将来的に `0.0`〜`1.0` の実数も受け付ける。
- `wires[].src`, `wires[].dst`: `[x, y]` の 2 要素配列。
- `wires[].kind`: `"positive"` または `"negative"`。

### バリデーション

JSON パース後、以下を検証する:

1. ワイヤの `src`, `dst` が `cells` に存在するセルを参照していること。
2. `initial` が 0 または 1 であること（通常モード時）。
3. 自己ループ（src == dst）がないこと。

エラーは `Result` で返し、具体的なエラーメッセージを含める。

### serde のデシリアライズ構造体

JSON 用の構造体とドメインモデルは分離する。

```rust
// io/json.rs

#[derive(Deserialize)]
pub struct CircuitJson {
    pub cells: Vec<CellJson>,
    pub wires: Vec<WireJson>,
}

#[derive(Deserialize)]
pub struct CellJson {
    pub x: i32,
    pub y: i32,
    pub initial: u8,
}

#[derive(Deserialize)]
pub struct WireJson {
    pub src: [i32; 2],
    pub dst: [i32; 2],
    pub kind: String,
}
```

`CircuitJson` → `Circuit` への変換メソッド (`TryFrom` or 専用関数) でバリデーションを行う。

### 出力フォーマット

各セルの 1〜100 tick 後の状態を出力する。

```json
{
  "ticks": [
    {
      "tick": 1,
      "cells": { "0,0": 0, "1,0": 1, "2,0": 0 }
    },
    {
      "tick": 2,
      "cells": { "0,0": 0, "1,0": 1, "2,0": 0 }
    }
  ]
}
```

- セルのキーは `"x,y"` 形式の文字列（JSON のキーは文字列である必要があるため）。
- 値は通常モードでは `0` or `1`。

### CLI インターフェース

```
USAGE:
    lgcell2-core [OPTIONS] [FILE]

ARGS:
    [FILE]    回路定義 JSON ファイル。省略時は標準入力から読み込み。

OPTIONS:
    -t, --ticks <N>    シミュレーションする tick 数 (デフォルト: 100)
    -h, --help         ヘルプを表示
```

- 外部クレートは最小限にする。引数パースは手動実装（clap 等は使わない）。
- 入力: ファイルパスまたは stdin から JSON を読み込む。
- 出力: stdout に結果 JSON を出力する。
- エラー: stderr にエラーメッセージを出力し、非ゼロ終了コードで終了する。

### 依存クレート

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### テスト方針

- **正常系**: 有効な JSON → `Circuit` へのパースを確認。
- **異常系**: 存在しないセル参照、不正な `kind` 値、自己ループ等でエラーが返ることを確認。
- **ラウンドトリップ**: 出力 JSON が期待通りのフォーマットであることを確認。
- **CLI**: stdin / ファイル入力の両方で動作することを確認（Feature-Fake test）。
