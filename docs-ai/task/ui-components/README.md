# UI コンポーネント追加

GUI 向けのインタラクティブ入力コンポーネント（toggle-btn, pulse-btn, push-btn）および出力コンポーネント（cell-light）を追加する。

作成日: 2026-04-10
ステータス: 設計完了（未実装）

## 背景・動機

現在の入力コンポーネントは `Generator`（tick ベースのパターン注入）のみであり、ユーザがリアルタイムに回路の入力を操作する手段がない。また、出力コンポーネントは `Tester`（期待値検証）のみであり、回路の状態を視覚的に表示するための手段がない。

GUI エディタで回路を操作可能にするため、対話的な入力コンポーネントと視覚的な出力コンポーネントを追加する。

## 新規コンポーネント一覧

### 入力コンポーネント

| コンポーネント | サイズ | デフォルト値 | 挙動 |
|---|---|---|---|
| `toggle-btn` | 1×1 | `false` (`DEFAULT_TOGGLE_BTN_VALUE`) | ボタンを押す度に出力が `true` / `false` に切り替わる |
| `pulse-btn` | 1×1 | `false` (`DEFAULT_PULSE_BTN_VALUE`) | ボタン押下時、1 tick だけ `true` を出力する |
| `push-btn` | 1×1 | `false` (`DEFAULT_PUSH_BTN_VALUE`) | ボタンを押している間だけ `true` を出力する |

### 出力コンポーネント

| コンポーネント | サイズ | 挙動 |
|---|---|---|
| `cell-light` | 1×1 | 対象セルが `true` のとき点灯する（視覚表示のみ） |

## 設計・方針

### 方針の要点

1. **GUI 依存の分離**: ボタンの「押す」「離す」はGUI/API呼び出しに依存する。シミュレータ側では「現在の状態」を保持・適用するのみ
2. **CUI での挙動**: インタラクティブコンポーネントは常にデフォルト値を出力する。CUI にはボタン操作のインターフェースがないため
3. **tick 境界での状態適用**: インタラクティブ入力の状態は tick の開始時（`apply_inputs`）に適用される。tick 途中の変更は次の tick に反映される

### 全体像

```
┌─ JSON / WasmCircuitInput ─┐
│  input: [                  │
│    { type: "toggle_btn", target: [0,0] }, │
│    { type: "generator", ... },            │
│  ]                         │
│  output: [                 │
│    { type: "cell_light", target: [1,0] }, │
│    { type: "tester", ... },               │
│  ]                         │
└────────────┬───────────────┘
             │ parse
             ▼
┌─ Circuit ──────────────────┐
│  inputs: Vec<Input>        │  Input::ToggleBtn(ToggleBtn)
│  outputs: Vec<Output>      │  Output::CellLight(CellLight)
└────────────┬───────────────┘
             │ construct
             ▼
┌─ SimulatorSimple ──────────┐
│  interactive_states:       │  HashMap<Pos, bool>
│    (0,0) → false           │
│                            │
│  apply_inputs():           │
│    Generator → value_at(tick)           │
│    ToggleBtn → interactive_states[pos]  │
│    PulseBtn  → interactive_states[pos]  │
│    PushBtn   → interactive_states[pos]  │
│    ※ 適用後、PulseBtn は自動リセット    │
└────────────┬───────────────┘
             │ WASM API
             ▼
┌─ WasmSimulator ────────────┐
│  set_interactive_input()   │  tick 境界で適用
│  get_interactive_inputs()  │  GUI 描画用メタデータ取得
│  get_output_components()   │  GUI 描画用メタデータ取得
└────────────────────────────┘
```

詳細設計は以下のサブドキュメントに記載:

- [data-model.md](data-model.md) — データモデル変更（コンポーネント構造体・enum 拡張）
- [simulation.md](simulation.md) — シミュレーションエンジン変更（インタラクティブ状態管理）
- [json-wasm.md](json-wasm.md) — JSON パーサ・WASM API 変更

## ステップ

1. データモデル追加（コンポーネント構造体・enum 拡張） → [data-model.md](data-model.md)
2. シミュレーションエンジン変更（インタラクティブ状態管理、PulseBtn リセット） → [simulation.md](simulation.md)
3. JSON パーサ変更（新コンポーネントの解析） → [json-wasm.md](json-wasm.md)
4. WASM API 変更（インタラクティブ入力 API、メタデータ取得 API） → [json-wasm.md](json-wasm.md)
5. CUI 動作確認（デフォルト値で動作すること）
6. テスト追加（Unit-Fake テスト）
