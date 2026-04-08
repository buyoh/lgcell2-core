# データモデルの変更

## 新規型

### SubCircuitJson（io/json.rs）

JSON パース用のサブ回路定義モデル。

```rust
#[derive(Debug, Deserialize)]
pub struct SubCircuitJson {
    pub wires: Vec<WireJson>,
    pub sub_input: Vec<[i32; 2]>,
    pub sub_output: Vec<[i32; 2]>,
    #[serde(default)]
    pub modules: Vec<ModuleJson>,
}
```

### ModuleJson（io/json.rs）

JSON パース用のモジュールインスタンスモデル。

```rust
#[derive(Debug, Deserialize)]
pub struct ModuleJson {
    #[serde(rename = "type")]
    pub module_type: String,
    pub sub_circuit: Option<String>,
    pub input: Vec<[i32; 2]>,
    pub output: Vec<[i32; 2]>,
}
```

### CircuitJson の拡張（io/json.rs）

```rust
pub struct CircuitJson {
    pub wires: Vec<WireJson>,
    #[serde(default)]
    pub input: Vec<InputJson>,
    #[serde(default)]
    pub output: Vec<OutputJson>,
    #[serde(default)]
    pub generators: Vec<GeneratorJson>,
    #[serde(default)]
    pub testers: Vec<TesterJson>,
    #[serde(default)]
    pub modules: Vec<ModuleJson>,              // 新規
    #[serde(default)]
    pub sub_circuits: HashMap<String, SubCircuitJson>,  // 新規
}
```

### ResolvedModule（circuit/ 配下に新規ファイル）

解決済みのモジュールインスタンス。サブ回路定義が Circuit として構築済み。

```rust
/// 解決済みモジュールインスタンス。
/// サブ回路の Circuit を保持し、入出力セルの親座標⇔ローカル座標のマッピングを提供する。
pub struct ResolvedModule {
    /// サブ回路の回路定義（ネストされたモジュールを含む）。
    circuit: Circuit,
    /// 親座標系での入力セル位置。
    input: Vec<Pos>,
    /// 親座標系での出力セル位置。
    output: Vec<Pos>,
    /// サブ回路ローカル座標系での入力インターフェースセル。
    sub_input: Vec<Pos>,
    /// サブ回路ローカル座標系での出力インターフェースセル。
    sub_output: Vec<Pos>,
}
```

## 既存型の変更

### Circuit

`modules` フィールドを追加:

```rust
pub struct Circuit {
    cells: BTreeSet<Pos>,
    wires: Vec<Wire>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    modules: Vec<ResolvedModule>,    // 新規
    incoming: HashMap<Pos, Vec<usize>>,
    sorted_cells: Vec<Pos>,
}
```

`Circuit::with_components` のシグネチャを拡張するか、新しいコンストラクタを追加する:

```rust
impl Circuit {
    pub fn with_modules(
        cells: BTreeSet<Pos>,
        wires: Vec<Wire>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        modules: Vec<ResolvedModule>,
    ) -> Result<Self, CircuitError>;
}
```

アクセサ追加:

```rust
impl Circuit {
    pub fn modules(&self) -> &[ResolvedModule];
}
```

### 構築時の追加検証

`Circuit::with_modules` で以下を追加検証する:

1. ポート列制約: 各モジュールの `input` が同一 x 座標・連続 y 座標であること
2. ポート列制約: 各モジュールの `output` が同一 x 座標・連続 y 座標であること
3. 各モジュールについて `output` の x > `input` の x
4. モジュールの出力セルが `incoming` に含まれないこと（入力ワイヤ禁止）
5. モジュールの出力セル間で座標の重複がないこと
6. モジュールの出力セルが Generator ターゲットでないこと

## エラー型の追加（base/error.rs）

`CircuitError` に以下のバリアントを追加:

```rust
pub enum CircuitError {
    // ... 既存バリアント ...

    #[error("module output {0} must not have incoming wires")]
    ModuleOutputHasIncomingWires(Pos),

    #[error("duplicate module output: {0}")]
    DuplicateModuleOutput(Pos),

    #[error("module output must come after all module inputs in lexicographic order")]
    ModuleOutputBeforeInput,

    #[error("port column constraint violated: ports must share same x and have contiguous y")]
    InvalidPortColumn,

    #[error("sub_input count mismatch: expected {expected}, got {actual}")]
    SubInputCountMismatch { expected: usize, actual: usize },

    #[error("sub_output count mismatch: expected {expected}, got {actual}")]
    SubOutputCountMismatch { expected: usize, actual: usize },

    #[error("sub_output must come after all sub_input in lexicographic order")]
    SubOutputBeforeSubInput,

    #[error("sub_input {0} must not have incoming wires within sub-circuit")]
    SubInputHasIncomingWires(Pos),
}
```

`ParseError` にサブ回路関連のバリアントを追加:

```rust
pub enum ParseError {
    // ... 既存バリアント ...

    #[error("sub-circuit not found: {0}")]
    SubCircuitNotFound(String),

    #[error("circular dependency detected in sub-circuits: {0}")]
    CircularDependency(String),
}
```

## 解決処理フロー（io/json.rs）

`CircuitJson` から `Circuit` への変換時に、サブ回路の解決を行う:

```
CircuitJson
  ├── wires → Wire[] (既存)
  ├── sub_circuits → 依存順にソート → 各定義を Circuit に変換
  └── modules → ResolvedModule[] に変換
       ├── sub_circuit の Circuit を取得
       ├── input/output の Pos 変換
       └── sub_input/sub_output のカウント検証
```

### 循環依存の検出

サブ回路の依存関係をグラフとして構築し、トポロジカルソートを実行する。ソートが失敗した場合は循環依存エラーを返す。

```
half_adder → (依存なし)
full_adder → half_adder
```

トポロジカルソート順に処理することで、依存先のサブ回路が常に先に構築される。
