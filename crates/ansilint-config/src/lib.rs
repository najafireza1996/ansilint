//! Typed configuration values, parsing and merge semantics.
//!
//! It must not: discover files or read the filesystem; discovery belongs to the engine.
#![forbid(unsafe_code)]

mod config;
mod error;
mod ignore;
mod profile;

pub use config::{merge, Config, ConfigOverrides, KindPattern, ParsedConfig, CONFIG_FILE_NAMES};
pub use error::ConfigError;
pub use ignore::{IgnoreEntry, IgnoreError, IgnoreFile, IGNORE_FILE_NAMES};
pub use profile::Profile;
