# View モードの対応

## 概要

View モード（TUI ビューモード）はサブ回路を含む回路に未対応とし、サブ回路を含む場合はエラーを返す。

## 背景

View モードは `HashMap<Pos, bool>` を受け取りグリッド表示する。サブ回路の内部状態は `TickOutput` に含まれないため、モジュール出力セルの値は表示されるが、内部の伝搬過程は不可視である。サブ回路のデバッグ表示や展開表示の設計が未定のため、初期実装ではサブ回路を含む回路を拒否する。

## 方針

### エラーチェックの実装箇所

`src/bin/lgcell2/view.rs` の `run_view_mode()` 関数の先頭で、`Circuit::modules()` が空でない場合にエラーを返す。

```rust
pub fn run_view_mode(circuit: Circuit) -> Result<(), String> {
    if !circuit.modules().is_empty() {
        return Err("view mode does not support circuits with sub-circuit modules".into());
    }
    let console = CrosstermConsole::new();
    run_view_loop(console, circuit)
}
```

### ViewRenderer の変更

`ViewRenderer` 自体には変更不要。`ViewRenderer` は `HashMap<Pos, bool>` のみを受け取る純粋な描画層であり、回路構造に関する知識を持たない。

### CLI の通常モード（JSON 出力）

`--view` なしの通常モード（JSON 出力）はサブ回路を含む回路を処理可能。`simulate_to_output_json()` は `Simulator` を使用し、階層的シミュレーションが正しく実行される。

## 将来の拡張

サブ回路対応の View モードでは以下の機能が考えられる:

- モジュールインスタンスの境界表示（入力列・出力列のハイライト）
- サブ回路内部へのドリルダウン表示
- モジュール出力値のラベル表示

これらは別タスクとして設計・実装する。
