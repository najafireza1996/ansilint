//! Configuration values and parsing.

use std::path::PathBuf;

use ansilint_yaml::{Document, Node, SourceFile};

use crate::error::ConfigError;
use crate::profile::Profile;

/// Configuration file names, in discovery order.
pub const CONFIG_FILE_NAMES: &[&str] = &[
    ".ansible-lint",
    ".ansible-lint.yml",
    ".ansible-lint.yaml",
    ".config/ansible-lint.yml",
    ".config/ansible-lint.yaml",
];

const IGNORED_KEYS: &[&str] = &[
    "rulesdir",
    "mock_modules",
    "mock_roles",
    "extra_vars",
    "skip_action_validation",
];

/// A user-defined file-kind pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindPattern {
    /// File kind name.
    pub kind: String,
    /// Glob relative to the project root.
    pub glob: String,
}

/// Typed configuration.
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors upstream configuration keys; an enum would not model them"
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Minimum profile.
    pub profile: Profile,
    /// Excluded paths, as written in the config.
    pub exclude_paths: Vec<PathBuf>,
    /// Tags hidden entirely.
    pub skip_list: Vec<String>,
    /// Tags reported but non-fatal.
    pub warn_list: Vec<String>,
    /// Opt-in rules to enable.
    pub enable_list: Vec<String>,
    /// If non-empty, only these tags are reported.
    pub tags: Vec<String>,
    /// Suppress non-finding output.
    pub quiet: bool,
    /// Treat warnings as failures.
    pub strict: bool,
    /// Verbosity level.
    pub verbosity: u8,
    /// Also write SARIF to this file.
    pub sarif_file: Option<PathBuf>,
    /// User file-kind patterns, prepended to the defaults.
    pub kinds: Vec<KindPattern>,
    /// Pattern for variable names.
    pub var_naming_pattern: Option<String>,
    /// Prefix for loop variables.
    pub loop_var_prefix: Option<String>,
    /// Prefix for task names.
    pub task_name_prefix: Option<String>,
    /// Maximum block nesting depth.
    pub max_block_depth: Option<u32>,
    /// Builtin collections allowed by `only-builtins`.
    pub only_builtins_allow_collections: Vec<String>,
    /// Builtin modules allowed by `only-builtins`.
    pub only_builtins_allow_modules: Vec<String>,
    /// Rules whose fixes may be written.
    pub write_list: Vec<String>,
    /// Linting is always offline; accepted for compatibility.
    pub offline: bool,
    /// Whether the default rule set is used.
    pub use_default_rules: bool,
    /// Additional supported Ansible versions.
    pub supported_ansible_also: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: Profile::Min,
            exclude_paths: Vec::new(),
            skip_list: Vec::new(),
            warn_list: Vec::new(),
            enable_list: Vec::new(),
            tags: Vec::new(),
            quiet: false,
            strict: false,
            verbosity: 0,
            sarif_file: None,
            kinds: Vec::new(),
            var_naming_pattern: None,
            loop_var_prefix: None,
            task_name_prefix: None,
            max_block_depth: None,
            only_builtins_allow_collections: Vec::new(),
            only_builtins_allow_modules: Vec::new(),
            write_list: Vec::new(),
            offline: true,
            use_default_rules: true,
            supported_ansible_also: Vec::new(),
        }
    }
}

/// The result of parsing a configuration file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedConfig {
    /// Typed values.
    pub config: Config,
    /// Keys that are valid but intentionally ignored.
    pub ignored_keys: Vec<String>,
    /// Keys that are not recognised.
    pub unknown_keys: Vec<String>,
}

/// Explicit CLI overrides; lists extend and scalars replace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigOverrides {
    /// Profile override.
    pub profile: Option<Profile>,
    /// Additional excluded paths.
    pub exclude_paths: Vec<PathBuf>,
    /// Additional skipped tags.
    pub skip_list: Vec<String>,
    /// Additional warning tags.
    pub warn_list: Vec<String>,
    /// Additional opt-in rules.
    pub enable_list: Vec<String>,
    /// Additional tag filters.
    pub tags: Vec<String>,
    /// Quiet override.
    pub quiet: Option<bool>,
    /// Strict override.
    pub strict: Option<bool>,
    /// Verbosity override.
    pub verbosity: Option<u8>,
    /// SARIF output file override.
    pub sarif_file: Option<PathBuf>,
    /// Additional writable rules.
    pub write_list: Vec<String>,
}

impl Config {
    /// Parses the first document of a configuration file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a known key has an invalid value.
    pub fn parse(documents: &[Document]) -> Result<ParsedConfig, ConfigError> {
        let Some(document) = documents.first() else {
            return Ok(default_parsed());
        };
        let Some(mapping) = document.root.as_mapping() else {
            if document.root.is_null() {
                return Ok(default_parsed());
            }
            return Err(ConfigError::new(
                document.root.span(),
                "configuration must be a mapping",
            ));
        };
        let mut parsed = default_parsed();
        for entry in &mapping.entries {
            let Some(key) = entry.key.as_str() else {
                return Err(ConfigError::new(
                    entry.key_span,
                    "configuration keys must be strings",
                ));
            };
            apply_key(&mut parsed, key, &entry.value)?;
        }
        parsed.ignored_keys.sort();
        parsed.unknown_keys.sort();
        Ok(parsed)
    }

    /// Parses configuration bytes through the shared YAML AST.
    ///
    /// # Errors
    ///
    /// Returns an error when YAML or a configuration value is invalid.
    pub fn from_bytes(text: &str) -> Result<ParsedConfig, ConfigError> {
        let source = SourceFile::new("<config>", text);
        let documents = ansilint_yaml::parse(&source)
            .map_err(|error| ConfigError::new(error.span, error.message))?;
        Self::parse(&documents)
    }
}

/// Applies CLI overrides to a configuration.
#[must_use]
pub fn merge(base: &Config, overrides: &ConfigOverrides) -> Config {
    let mut merged = base.clone();
    if let Some(profile) = overrides.profile {
        merged.profile = profile;
    }
    if let Some(quiet) = overrides.quiet {
        merged.quiet = quiet;
    }
    if let Some(strict) = overrides.strict {
        merged.strict = strict;
    }
    if let Some(verbosity) = overrides.verbosity {
        merged.verbosity = verbosity;
    }
    if let Some(sarif_file) = &overrides.sarif_file {
        merged.sarif_file = Some(sarif_file.clone());
    }
    merged
        .exclude_paths
        .extend(overrides.exclude_paths.iter().cloned());
    merged.skip_list.extend(overrides.skip_list.iter().cloned());
    merged.warn_list.extend(overrides.warn_list.iter().cloned());
    merged
        .enable_list
        .extend(overrides.enable_list.iter().cloned());
    merged.tags.extend(overrides.tags.iter().cloned());
    merged
        .write_list
        .extend(overrides.write_list.iter().cloned());
    merged
}

fn default_parsed() -> ParsedConfig {
    ParsedConfig {
        config: Config::default(),
        ignored_keys: Vec::new(),
        unknown_keys: Vec::new(),
    }
}

fn apply_key(parsed: &mut ParsedConfig, key: &str, value: &Node) -> Result<(), ConfigError> {
    match key {
        "profile" => parsed.config.profile = profile_value(value)?,
        "exclude_paths" => parsed.config.exclude_paths = path_list(value)?,
        "skip_list" => parsed.config.skip_list = string_list(value)?,
        "warn_list" => parsed.config.warn_list = string_list(value)?,
        "enable_list" => parsed.config.enable_list = string_list(value)?,
        "tags" => parsed.config.tags = string_list(value)?,
        "quiet" => parsed.config.quiet = bool_value(value)?,
        "strict" => parsed.config.strict = bool_value(value)?,
        "verbosity" => parsed.config.verbosity = u8_value(value)?,
        "sarif_file" => parsed.config.sarif_file = Some(PathBuf::from(string_value(value)?)),
        "kinds" => parsed.config.kinds = kind_list(value)?,
        "var_naming_pattern" => {
            parsed.config.var_naming_pattern = Some(string_value(value)?.to_string());
        }
        "loop_var_prefix" => parsed.config.loop_var_prefix = Some(string_value(value)?.to_string()),
        "task_name_prefix" => {
            parsed.config.task_name_prefix = Some(string_value(value)?.to_string());
        }
        "max_block_depth" => parsed.config.max_block_depth = Some(u32_value(value)?),
        "only_builtins_allow_collections" => {
            parsed.config.only_builtins_allow_collections = string_list(value)?;
        }
        "only_builtins_allow_modules" => {
            parsed.config.only_builtins_allow_modules = string_list(value)?;
        }
        "write_list" => parsed.config.write_list = string_list(value)?,
        "offline" => parsed.config.offline = bool_value(value)?,
        "use_default_rules" => parsed.config.use_default_rules = bool_value(value)?,
        "supported_ansible_also" => parsed.config.supported_ansible_also = string_list(value)?,
        ignored if IGNORED_KEYS.contains(&ignored) => {
            parsed.ignored_keys.push(ignored.to_string());
        }
        unknown => parsed.unknown_keys.push(unknown.to_string()),
    }
    Ok(())
}

fn string_value(node: &Node) -> Result<&str, ConfigError> {
    node.as_str()
        .ok_or_else(|| ConfigError::new(node.span(), "expected a string"))
}

fn bool_value(node: &Node) -> Result<bool, ConfigError> {
    let value = string_value(node)?;
    if value.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(ConfigError::new(
            node.span(),
            format!("expected `true` or `false`, found `{value}`"),
        ))
    }
}

fn u32_value(node: &Node) -> Result<u32, ConfigError> {
    let value = string_value(node)?;
    value.parse().map_err(|_| {
        ConfigError::new(
            node.span(),
            format!("expected a positive integer, found `{value}`"),
        )
    })
}

fn u8_value(node: &Node) -> Result<u8, ConfigError> {
    let value = string_value(node)?;
    let parsed = value.parse::<u32>().map_err(|_| {
        ConfigError::new(
            node.span(),
            format!("expected a small integer, found `{value}`"),
        )
    })?;
    u8::try_from(parsed)
        .map_err(|_| ConfigError::new(node.span(), format!("verbosity `{parsed}` is too large")))
}

fn profile_value(node: &Node) -> Result<Profile, ConfigError> {
    let value = string_value(node)?;
    value
        .parse()
        .map_err(|()| ConfigError::new(node.span(), format!("unknown profile `{value}`")))
}

fn string_list(node: &Node) -> Result<Vec<String>, ConfigError> {
    if let Some(sequence) = node.as_sequence() {
        return sequence
            .items
            .iter()
            .map(|item| string_value(item).map(str::to_string))
            .collect();
    }
    Ok(vec![string_value(node)?.to_string()])
}

fn path_list(node: &Node) -> Result<Vec<PathBuf>, ConfigError> {
    Ok(string_list(node)?.into_iter().map(PathBuf::from).collect())
}

fn kind_list(node: &Node) -> Result<Vec<KindPattern>, ConfigError> {
    let Some(sequence) = node.as_sequence() else {
        return Err(ConfigError::new(node.span(), "`kinds` must be a list"));
    };
    let mut patterns = Vec::new();
    for item in &sequence.items {
        let Some(mapping) = item.as_mapping() else {
            return Err(ConfigError::new(item.span(), "each kind must be a mapping"));
        };
        let kind = required_string(mapping.get("kind"), "kind")?;
        let glob = required_string(mapping.get("glob"), "glob")?;
        patterns.push(KindPattern {
            kind: kind.to_string(),
            glob: glob.to_string(),
        });
    }
    Ok(patterns)
}

fn required_string<'a>(node: Option<&'a Node>, key: &str) -> Result<&'a str, ConfigError> {
    let Some(node) = node else {
        return Err(ConfigError::new(
            ansilint_core::Span::empty(0),
            format!("kind entry is missing `{key}`"),
        ));
    };
    string_value(node)
}

#[cfg(test)]
mod tests;
