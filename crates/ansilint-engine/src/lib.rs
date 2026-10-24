//! Orchestration of discovery and loading.
//!
//! It must not: implement rule logic or output formatting itself.
#![forbid(unsafe_code)]

mod discovery;
mod error;
mod loader;

pub use error::EngineError;
pub use loader::DEFAULT_MAX_FILE_BYTES;
