mod legacy;
mod simulator;
mod types;

pub use legacy::{simulate, simulate_n};
pub use simulator::WasmSimulator;
pub use types::{WasmCellState, WasmCircuitInput, WasmStepRunResult, WasmTickResult};
