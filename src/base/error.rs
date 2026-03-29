/// Errors that occur during circuit structure validation.
#[derive(Debug, thiserror::Error)]
pub enum CircuitError {
    #[error("self-loop wire is not allowed: src={}, dst={}", .src, .dst)]
    SelfLoop {
        src: crate::base::Pos,
        dst: crate::base::Pos,
    },

    #[error("wire src does not exist in cells: {0}")]
    WireSrcNotFound(crate::base::Pos),

    #[error("wire dst does not exist in cells: {0}")]
    WireDstNotFound(crate::base::Pos),

    #[error("duplicate wire is not allowed: src={}, dst={}", .src, .dst)]
    DuplicateWire {
        src: crate::base::Pos,
        dst: crate::base::Pos,
    },

    #[error("input target {0} must not have incoming wires")]
    InputTargetHasIncomingWires(crate::base::Pos),

    #[error("duplicate input target is not allowed: {0}")]
    DuplicateInputTarget(crate::base::Pos),

    #[error("generator pattern must not be empty: {0}")]
    EmptyGeneratorPattern(crate::base::Pos),

    #[error("duplicate output target is not allowed: {0}")]
    DuplicateOutputTarget(crate::base::Pos),

    #[error("tester expected pattern must not be empty: {0}")]
    EmptyTesterPattern(crate::base::Pos),
}

/// Errors that occur during format conversion (wire kind, pattern, etc.).
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("wire kind must be positive or negative: {0}")]
    InvalidWireKind(String),

    #[error("invalid pattern character: '{0}' (expected '0' or '1')")]
    InvalidPatternChar(char),

    #[error("invalid expected pattern character: '{0}' (expected '0', '1', or 'x')")]
    InvalidExpectedPatternChar(char),
}

/// Errors that occur during JSON parsing and circuit construction.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Format(#[from] FormatError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Circuit(#[from] CircuitError),
}

/// Errors that occur during simulation execution.
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("unknown cell at {0}")]
    UnknownCell(crate::base::Pos),
}
