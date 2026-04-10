# シミュレーションエンジン変更

UI コンポーネントのインタラクティブ状態管理に伴うシミュレーションエンジンの変更を規定する。

## 概要

インタラクティブ入力コンポーネント（ToggleBtn, PulseBtn, PushBtn）の値は tick に依存せず、外部からの操作によって決定される。シミュレータ内に `interactive_states: HashMap<Pos, bool>` を追加し、`apply_inputs` で Generator とインタラクティブコンポーネントを区別して処理する。

## SimulatorSimple の変更

### 新規フィールド

```rust
pub struct SimulatorSimple {
    // ... 既存フィールド ...

    /// インタラクティブ入力コンポーネントの現在の状態。
    /// キー: 対象セルの Pos、値: 現在の出力値。
    interactive_states: HashMap<Pos, bool>,
}
```

### 初期化

`SimulatorSimple::new` で、回路内のインタラクティブ入力コンポーネントを走査し、デフォルト値で `interactive_states` を初期化する:

```rust
impl SimulatorSimple {
    pub fn new(circuit: Circuit) -> Self {
        // ... 既存の初期化 ...

        let mut interactive_states = HashMap::new();
        for input in circuit.inputs() {
            match input {
                Input::ToggleBtn(t) => {
                    interactive_states.insert(t.target(), t.default_value());
                }
                Input::PulseBtn(p) => {
                    interactive_states.insert(p.target(), p.default_value());
                }
                Input::PushBtn(p) => {
                    interactive_states.insert(p.target(), p.default_value());
                }
                Input::Generator(_) => {}
            }
        }

        SimulatorSimple {
            // ... 既存フィールド ...
            interactive_states,
        }
    }
}
```

### apply_inputs の変更

```rust
fn apply_inputs(&mut self) {
    for input in self.circuit.inputs() {
        let target = input.target();
        if let Some(&index) = self.cell_pos_to_index.get(&target) {
            if input.is_interactive() {
                // インタラクティブコンポーネント: interactive_states から値を取得
                let value = self.interactive_states
                    .get(&target)
                    .copied()
                    .unwrap_or(false);
                self.cell_values[index] = value;
            } else {
                // Generator: tick ベースの値を使用
                self.cell_values[index] = input.value_at(self.tick);
            }
        }
    }

    // PulseBtn は 1 tick だけ true を出力するため、適用後にリセットする。
    // これにより次の tick では false になる（再度トリガーされない限り）。
    for input in self.circuit.inputs() {
        if input.is_pulse() {
            self.interactive_states.insert(input.target(), false);
        }
    }
}
```

**PulseBtn のリセットタイミングが重要**:
- `apply_inputs` でまず現在の `interactive_states` の値をセルに適用する
- その後、PulseBtn の `interactive_states` を `false` にリセットする
- 結果: トリガーされた tick では `true` が適用され、次の tick では `false` が適用される

### Simulator トレイトの拡張

```rust
pub trait Simulator {
    // ... 既存メソッド ...

    /// インタラクティブ入力コンポーネントの状態を設定する。
    ///
    /// tick 境界で適用される。`is_updating() == true` の間に呼び出した場合、
    /// 現在の tick には反映されず、次の tick の開始時に適用される。
    fn set_interactive_input(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError>;

    /// インタラクティブ入力コンポーネントの現在の状態を取得する。
    fn get_interactive_states(&self) -> &HashMap<Pos, bool>;
}
```

### SimulatorSimple の実装

```rust
fn set_interactive_input(&mut self, pos: Pos, value: bool) -> Result<(), SimulationError> {
    if self.interactive_states.contains_key(&pos) {
        self.interactive_states.insert(pos, value);
        Ok(())
    } else {
        Err(SimulationError::NotInteractiveInput(pos))
    }
}

fn get_interactive_states(&self) -> &HashMap<Pos, bool> {
    &self.interactive_states
}
```

## CUI での挙動

CUI シミュレーション（`src/bin/lgcell2/`）では、インタラクティブコンポーネントに対する外部操作は行われない。`interactive_states` は初期化時のデフォルト値（全て `false`）のまま維持される。

CUI 上のシミュレーションでは `set_interactive_input` を呼ばないため:
- ToggleBtn: 常に `false`（`DEFAULT_TOGGLE_BTN_VALUE`）を出力
- PulseBtn: 常に `false`（`DEFAULT_PULSE_BTN_VALUE`）を出力
- PushBtn: 常に `false`（`DEFAULT_PUSH_BTN_VALUE`）を出力

CUI の既存コード変更は不要。

## CellLight のシミュレーション上の扱い

`CellLight` はシミュレーション動作に影響を与えない。`Output::CellLight` は `verify_testers` では無視される（`Output::Tester` のみ検証対象）。既存の `verify_testers` 実装は `match` で `Output::Tester` のみを処理しているため、`Output::CellLight` アームを追加して何もしないだけでよい。

```rust
fn verify_testers(&self) -> Vec<TesterResult> {
    // ...
    for output in self.circuit.outputs() {
        match output {
            Output::Tester(tester) => { /* 既存のテスター検証 */ }
            Output::CellLight(_) => { /* シミュレーション動作なし */ }
        }
    }
    // ...
}
```

## エラー型の追加

`SimulationError` に新しいバリアントを追加:

```rust
pub enum SimulationError {
    UnknownCell(Pos),
    /// 指定された座標にインタラクティブ入力コンポーネントが存在しない。
    NotInteractiveInput(Pos),
}
```

## テスト方針

### Unit-Fake テスト

- `ToggleBtn` / `PulseBtn` / `PushBtn` の `default_value` が正しいこと
- `SimulatorSimple` の `interactive_states` がデフォルト値で初期化されること
- `set_interactive_input` で値が変更されること
- `set_interactive_input` で存在しない Pos を指定した場合にエラーが返ること
- `ToggleBtn` を含む回路で `set_interactive_input(pos, true)` → `tick()` → セル値が `true` になること
- `PulseBtn` をトリガーした場合、1 tick だけ `true` が出力され、次の tick では `false` に戻ること
- `PushBtn` を `true` にした場合、`true` の間はセル値が `true`、`false` にしたら `false` に戻ること
- `CellLight` を含む回路で、シミュレーションが正常に動作すること（verify_testers に影響しないこと）
- インタラクティブコンポーネントなしの回路では `interactive_states` が空であること
