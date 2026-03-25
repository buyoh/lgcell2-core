/// Errors that occur during circuit structure validation.
#[derive(Debug, thiserror::Error)]
pub enum CircuitError {
    #[error("self-loop wire is not allowed: src=({}, {}), dst=({}, {})", .src.x, .src.y, .dst.x, .dst.y)]
    SelfLoop { src: crate::circuit::Pos, dst: crate::circuit::Pos },

    #[error("wire src does not exist in cells: ({}, {})", .0.x, .0.y)]
    WireSrcNotFound(crate::circuit::Pos),

    #[error("wire dst does not exist in cells: ({}, {})", .0.x, .0.y)]
    WireDstNotFound(crate::circuit::Pos),

    #[error("duplicate wire is not allowed: src=({}, {}), dst=({}, {})", .src.x, .src.y, .dst.x, .dst.y)]
    DuplicateWire { src: crate::circuit::Pos, dst: crate::circuit::Pos },

    #[error("generator target ({}, {}) must not have incoming wires", .0.x, .0.y)]
    GeneratorTargetHasIncomingWires(crate::circuit::Pos),

    #[error("duplicate generator target is not allowed: ({}, {})", .0.x, .0.y)]
    DuplicateGeneratorTarget(crate::circuit::Pos),

    #[error("generator pattern must not be empty: ({}, {})", .0.x, .0.y)]
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
    #[error("unknown cell at ({}, {})", .0.x, .0.y)]
    UnknownCell(crate::circuit::Pos),
}
