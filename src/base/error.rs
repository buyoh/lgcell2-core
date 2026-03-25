/// Errors that occur during circuit structure validation.
#[derive(Debug, thiserror::Error)]
pub enum CircuitError {
    #[error("self-loop wire is not allowed: src={}, dst={}", .src, .dst)]
    SelfLoop { src: crate::circuit::Pos, dst: crate::circuit::Pos },

    #[error("wire src does not exist in cells: {0}")]
    WireSrcNotFound(crate::circuit::Pos),

    #[error("wire dst does not exist in cells: {0}")]
    WireDstNotFound(crate::circuit::Pos),

    #[error("duplicate wire is not allowed: src={}, dst={}", .src, .dst)]
    DuplicateWire { src: crate::circuit::Pos, dst: crate::circuit::Pos },

    #[error("generator target {0} must not have incoming wires")]
    GeneratorTargetHasIncomingWires(crate::circuit::Pos),

    #[error("duplicate generator target is not allowed: {0}")]
    DuplicateGeneratorTarget(crate::circuit::Pos),

    #[error("generator pattern must not be empty: {0}")]
    EmptyGeneratorPattern(crate::circuit::Pos),
}

/// Errors that occur during JSON parsing and circuit construction.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("wire kind must be positive or negative: {0}")]
    InvalidWireKind(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Circuit(#[from] CircuitError),
}

/// Errors that occur during simulation execution.
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("unknown cell at {0}")]
    UnknownCell(crate::circuit::Pos),
}
