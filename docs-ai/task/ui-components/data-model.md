# データモデル変更

UI コンポーネント追加に伴うデータモデルの変更を規定する。

## 新規構造体

### ToggleBtn（`src/circuit/input_com/toggle_btn.rs`）

```rust
/// トグルボタン入力コンポーネント。
///
/// ボタンを押す度に出力が true / false に切り替わる。
/// シミュレータ内のインタラクティブ状態で値が管理される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleBtn {
    target: Pos,
}

/// トグルボタンのデフォルト出力値。
pub const DEFAULT_TOGGLE_BTN_VALUE: bool = false;

impl ToggleBtn {
    pub fn new(target: Pos) -> Self {
        Self { target }
    }

    pub fn target(&self) -> Pos {
        self.target
    }

    pub fn default_value(&self) -> bool {
        DEFAULT_TOGGLE_BTN_VALUE
    }
}
```

### PulseBtn（`src/circuit/input_com/pulse_btn.rs`）

```rust
/// パルスボタン入力コンポーネント。
///
/// ボタン押下時、1 tick だけ true を出力する。
/// シミュレータが apply_inputs 後に自動的に false にリセットする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseBtn {
    target: Pos,
}

/// パルスボタンのデフォルト出力値。
pub const DEFAULT_PULSE_BTN_VALUE: bool = false;

impl PulseBtn {
    pub fn new(target: Pos) -> Self {
        Self { target }
    }

    pub fn target(&self) -> Pos {
        self.target
    }

    pub fn default_value(&self) -> bool {
        DEFAULT_PULSE_BTN_VALUE
    }
}
```

### PushBtn（`src/circuit/input_com/push_btn.rs`）

```rust
/// プッシュボタン入力コンポーネント。
///
/// ボタンを押している間だけ true を出力する。
/// シミュレータ内のインタラクティブ状態で値が管理される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushBtn {
    target: Pos,
}

/// プッシュボタンのデフォルト出力値。
pub const DEFAULT_PUSH_BTN_VALUE: bool = false;

impl PushBtn {
    pub fn new(target: Pos) -> Self {
        Self { target }
    }

    pub fn target(&self) -> Pos {
        self.target
    }

    pub fn default_value(&self) -> bool {
        DEFAULT_PUSH_BTN_VALUE
    }
}
```

### CellLight（`src/circuit/output_com/cell_light.rs`）

```rust
/// セルライト出力コンポーネント。
///
/// 対象セルの値が true のとき点灯する視覚的なコンポーネント。
/// シミュレーション動作への影響はない。GUI がセルの種類を判別するためのメタデータ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellLight {
    target: Pos,
}

impl CellLight {
    pub fn new(target: Pos) -> Self {
        Self { target }
    }

    pub fn target(&self) -> Pos {
        self.target
    }
}
```

## Input / Output enum の拡張

### Input enum（`src/circuit/component.rs`）

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Generator(Generator),
    ToggleBtn(ToggleBtn),
    PulseBtn(PulseBtn),
    PushBtn(PushBtn),
}
```

`InputComponent` トレイトの実装で、`value_at` は以下のように分岐する:

```rust
impl InputComponent for Input {
    fn target(&self) -> Pos {
        match self {
            Input::Generator(g) => g.target(),
            Input::ToggleBtn(t) => t.target(),
            Input::PulseBtn(p) => p.target(),
            Input::PushBtn(p) => p.target(),
        }
    }

    fn value_at(&self, tick: u64) -> bool {
        match self {
            Input::Generator(g) => g.value_at(tick),
            // インタラクティブコンポーネントは tick に依存しないため
            // デフォルト値を返す。実際の値はシミュレータの
            // interactive_states から取得される。
            Input::ToggleBtn(t) => t.default_value(),
            Input::PulseBtn(p) => p.default_value(),
            Input::PushBtn(p) => p.default_value(),
        }
    }
}
```

**注意**: `value_at` がデフォルト値を返す設計にする理由:
- CUI シミュレーションでは `apply_inputs` が `value_at(tick)` を呼ぶため、デフォルト値がそのまま使われる
- WASM シミュレーションでは `apply_inputs` 内でインタラクティブコンポーネントを特別扱いし、`interactive_states` から値を取得する（詳細は [simulation.md](simulation.md) を参照）

### Output enum（`src/circuit/component.rs`）

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Tester(Tester),
    CellLight(CellLight),
}

impl OutputComponent for Output {
    fn target(&self) -> Pos {
        match self {
            Output::Tester(t) => t.target(),
            Output::CellLight(c) => c.target(),
        }
    }
}
```

## インタラクティブ判定ヘルパー

`Input` enum にインタラクティブコンポーネントかどうかを判定するメソッドを追加:

```rust
impl Input {
    /// インタラクティブ入力コンポーネント（GUI 操作依存）かどうかを返す。
    pub fn is_interactive(&self) -> bool {
        matches!(self, Input::ToggleBtn(_) | Input::PulseBtn(_) | Input::PushBtn(_))
    }

    /// PulseBtn かどうかを返す（apply_inputs 後の自動リセットに使用）。
    pub fn is_pulse(&self) -> bool {
        matches!(self, Input::PulseBtn(_))
    }
}
```

## ファイル構成

```
src/circuit/
  input_com/
    mod.rs           # ToggleBtn, PulseBtn, PushBtn を追加 re-export
    generator.rs     # 既存
    toggle_btn.rs    # 新規
    pulse_btn.rs     # 新規
    push_btn.rs      # 新規
  output_com/
    mod.rs           # CellLight を追加 re-export
    tester.rs        # 既存
    cell_light.rs    # 新規
  component.rs       # Input/Output enum 拡張
```
