use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("path is not a regular file: {0}")]
    InvalidFile(String),
    #[error("unsupported dataset type: {0}")]
    UnsupportedDatasetType(String),
    #[error("variable selection is required for this operation")]
    MissingVariable,
    #[error("invalid variable: {0}")]
    InvalidVariable(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid compare request: {0}")]
    InvalidCompare(String),
    #[error("provider not configured: {0}")]
    ProviderNotConfigured(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("agent error: {0}")]
    Agent(String),
    #[error("plan error: {0}")]
    Plan(String),
    #[error("capability error: {0}")]
    Capability(String),
    #[error("policy error: {0}")]
    Policy(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("i/o error: {0}")]
    Io(String),
    #[error("session error: {0}")]
    Session(String),
    #[error("output error: {0}")]
    Output(String),
}
