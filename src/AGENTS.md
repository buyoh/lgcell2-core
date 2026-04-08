# src/ ソースコード構成ガイド

## 概要

LGCELL2-Core は、論理回路をグリッドベースの有向グラフとして表現し、ステップ実行可能なシミュレーションを提供する Rust ライブラリである。セル（ノード）とワイヤ（辺）で回路を構成し、OR ロジックによる信号合成で NAND ゲート（万能ゲート）を実現する。

wasm ライブラリとしても native バイナリとしても動作する。

## ディレクトリ構造と役割

```
src/
├── lib.rs                  # クレートルート（モジュール公開）
├── bin/
│   └── lgcell2/
│       └── main.rs         # CLI エントリポイント
├── circuit/                # 回路データモデル（最下層・依存なし）
│   ├── mod.rs
│   ├── pos.rs              # グリッド座標 Pos
│   ├── pos_tests.rs
│   ├── wire.rs             # ワイヤ定義 Wire, WireKind
│   ├── wire_tests.rs
│   ├── circuit.rs          # 回路全体 Circuit
│   └── circuit_tests.rs
├── simulation/             # シミュレーションエンジン（circuit に依存）
│   ├── mod.rs
│   ├── engine.rs           # Simulator trait と型定義
│   ├── engine_simple.rs    # SimulatorSimple 実装
│   └── engine_tests.rs
└── io/                     # JSON 入出力（circuit, simulation に依存）
    ├── mod.rs
    ├── json.rs             # JSON パース・出力
    └── json_tests.rs
```

## モジュール間の依存関係

```
bin/lgcell2/main.rs
    ↓ (全モジュールを統合)
io::json ──→ circuit, simulation
view     ──→ circuit (HashMap<Pos,bool> を受け取る)
wasm_api ──→ circuit, simulation
simulation ──→ circuit
circuit ──→ (外部依存なし)
```

## 各モジュールの詳細

### `circuit/` — 回路データモデル

最下層モジュール。他の内部モジュールへの依存はない。

- **`Pos`**: グリッド座標 `(x: i32, y: i32)`。`Ord` は `(x, y)` の辞書順で導出され、この順序がシミュレーションの処理順序を決定する。
- **`WireKind`**: ワイヤの極性。`Positive`（そのまま伝搬）/ `Negative`（反転伝搬）。
- **`Wire`**: 有向辺。`src` → `dst` の信号伝搬を定義。`propagate()` で極性に応じた値変換を行う。
- **`Circuit`**: 回路全体の不変定義。`BTreeSet<Pos>` でセルをソート状態で保持し、`incoming: HashMap<Pos, Vec<usize>>` で dst → ワイヤインデックスの逆引きを事前計算する。構築時に self-loop 禁止・端点存在検証を行う。

### `simulation/` — シミュレーションエンジン

`circuit` モジュールに依存する。

- **`Simulator`**: シミュレーションエンジンの公開インターフェース（trait）。
- **`SimulatorSimple`**: `Simulator` の標準実装。`prev_cell_values: Vec<bool>`（前 tick の全セル値）と `cell_values: Vec<bool>`（現在の全セル値）を保持し、in-place で更新する。
  - `step()`: セル 1 個分を処理し `StepResult` を返す
  - `tick()`: 1 tick 完了まで処理
  - `run(n)`: n tick 実行
  - `run_with_snapshots(n)`: 各 tick の `TickOutput` を収集
  - `run_with_verification(n)`: 各 tick のテスター検証結果を収集
  - `set_cell()`: セル値を更新し、出力キャッシュも同期する
  - `last_output()`: 直近の `TickOutput` を返す
  - `replay_tick()`: 現在状態から出力キャッシュを再構築する
- **`StepResult`**: `Continue`（tick 内に未処理セルあり）/ `TickComplete`（tick 完了）
- **`TickOutput`**: 完了済み tick の 0-based 番号とセル値の `HashMap<Pos, bool>`。`OutputFormat` に応じて全セルまたは矩形領域内のセルのみを含む
- **`OutputFormat`**: `AllCell`（全セル）/ `ViewPort(Vec<Rect>)`（矩形領域内のみ）

**シミュレーション伝搬ルール:**
- セルは `Pos` の辞書順 `(x, y)` で処理される
- ワイヤ遅延は座標順序で自動決定:
  - `dst < src`（辞書順） → 遅延伝搬（`prev_cell_values[src]` から前 tick の値を取得）
  - `dst >= src`（辞書順） → 即時伝搬（`cell_values[src]` を直接参照）
- セルの値 = 全入力ワイヤの伝搬値の OR（`max()`）。入力なしの場合は `prev_cell_values` から前 tick の値を取得

### `io/` — JSON 入出力

`circuit` と `simulation` に依存する。

- **`CircuitJson` / `WireJson`**: JSON スキーマに対応する入力モデル。内部モデル `Circuit` とは `TryFrom` で変換し、スキーマ変更時の影響を隔離する。
- **`SimulationOutputJson` / `TickStateJson`**: シミュレーション結果の出力 JSON モデル。
- **`parse_circuit_json()`**: JSON 文字列 → `Circuit`。ワイヤの端点からセルを自動推論する。
- **`simulate_to_output_json()`**: `Circuit` + tick 数 → 出力モデルへの変換。
- **`output_json_to_string()`**: 出力モデルの JSON 文字列化。

### `bin/lgcell2/main.rs` — CLI エントリポイント

`clap` による CLI パーサ。ファイルまたは stdin から回路定義 JSON を読み込み、シミュレーション結果を stdout に JSON 出力する。

## 主要な設計パターン

1. **イミュータブル Circuit + ミュータブル SimulatorSimple**: `Circuit` は構築後不変でスレッド安全。`SimulatorSimple` が変更可能な状態を管理する。
2. **座標順序ベースのシミュレーション**: `Pos` の `Ord` が処理順を決定し、ワイヤ遅延も自動決定される。明示的な遅延宣言は不要。
3. **Cell ベースの状態管理**: `prev_cell_values` と `cell_values` の 2 つの `Vec<bool>` で全セルの前 tick 値と現 tick 値を保持し、`complete_tick()` で一括コピーする。
4. **ステップ実行エンジン**: `cell_index` で処理位置を保持する明示的なステートマシン。async/await を使わずに Web 上での中断・再開に対応。
5. **JSON スキーマの分離**: `CircuitJson` ↔ `Circuit` を `TryFrom` で変換し、スキーマと内部実装を隔離。

## テスト構成

- 各モジュールに `*_tests.rs` を配置（Unit-Fake テスト）
- `tests/` ディレクトリに統合テスト（Feature-Fake テスト）
  - `half_adder.rs`: 半加算器の真理値表検証・ステートレス動作確認
- 外部リソースへのアクセスやファイル作成を伴うテストは必ずモックを使用
- Mock ライブラリによるメソッド差し替えは禁止。スタブ・依存性注入を使う
