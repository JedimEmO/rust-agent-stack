use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during OpenRPC to Bruno conversion
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Failed to read OpenRPC file at {path}: {source}")]
    InputFileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse OpenRPC specification: {0}")]
    OpenRpcParse(#[from] serde_json::Error),

    #[error("Invalid OpenRPC specification: {0}")]
    OpenRpcValidation(String),

    #[error("Failed to create output directory {path}: {source}")]
    OutputDirCreate {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Output directory {path} already exists (use --force to overwrite)")]
    OutputDirExists { path: PathBuf },

    #[error("Failed to write Bruno file {path}: {source}")]
    BrunoFileWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Invalid base URL: {0}")]
    InvalidBaseUrl(String),

    #[error("OpenRPC specification has no methods defined")]
    NoMethodsDefined,

    #[error("Failed to resolve OpenRPC references: {0}")]
    ReferenceResolution(String),

    #[error("Unsupported OpenRPC feature: {0}")]
    UnsupportedFeature(String),
}
