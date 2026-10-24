//! File kinds recognised by the model.

/// The kind of Ansible content a file holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum FileKind {
    /// A playbook, usually a list of plays.
    Playbook,
    /// A reusable task file.
    Tasks,
    /// A handler file.
    Handlers,
    /// Role or play variables.
    Vars,
    /// Role defaults.
    Defaults,
    /// Role `meta/main.yml`.
    Meta,
    /// `meta/runtime.yml`.
    MetaRuntime,
    /// `galaxy.yml`.
    Galaxy,
    /// `requirements.yml`.
    Requirements,
    /// An inventory file.
    Inventory,
    /// `meta/argument_specs.yml`.
    RoleArgSpec,
    /// A Molecule scenario file.
    Molecule,
    /// An execution-environment definition.
    ExecutionEnvironment,
    /// A changelog fragment or file.
    Changelog,
    /// Generic YAML that is not Ansible-specific.
    Yaml,
    /// The kind could not be determined.
    Unknown,
}

impl FileKind {
    /// Returns the stable lowercase name used in configuration.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Playbook => "playbook",
            Self::Tasks => "tasks",
            Self::Handlers => "handlers",
            Self::Vars => "vars",
            Self::Defaults => "defaults",
            Self::Meta => "meta",
            Self::MetaRuntime => "meta-runtime",
            Self::Galaxy => "galaxy",
            Self::Requirements => "requirements",
            Self::Inventory => "inventory",
            Self::RoleArgSpec => "role-arg-spec",
            Self::Molecule => "molecule",
            Self::ExecutionEnvironment => "execution-environment",
            Self::Changelog => "changelog",
            Self::Yaml => "yaml",
            Self::Unknown => "unknown",
        }
    }

    /// Parses a configured kind name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "playbook" => Some(Self::Playbook),
            "tasks" => Some(Self::Tasks),
            "handlers" => Some(Self::Handlers),
            "vars" => Some(Self::Vars),
            "defaults" => Some(Self::Defaults),
            "meta" => Some(Self::Meta),
            "meta-runtime" => Some(Self::MetaRuntime),
            "galaxy" => Some(Self::Galaxy),
            "requirements" => Some(Self::Requirements),
            "inventory" => Some(Self::Inventory),
            "role-arg-spec" => Some(Self::RoleArgSpec),
            "molecule" => Some(Self::Molecule),
            "execution-environment" => Some(Self::ExecutionEnvironment),
            "changelog" => Some(Self::Changelog),
            "yaml" => Some(Self::Yaml),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns `true` when the file contains a list of tasks or handlers.
    #[must_use]
    pub const fn is_task_list(self) -> bool {
        matches!(self, Self::Tasks | Self::Handlers)
    }
}

#[cfg(test)]
mod tests {
    use super::FileKind;

    #[test]
    fn names_round_trip() {
        for kind in [
            FileKind::Playbook,
            FileKind::Tasks,
            FileKind::Handlers,
            FileKind::Vars,
            FileKind::Defaults,
            FileKind::Meta,
            FileKind::MetaRuntime,
            FileKind::Galaxy,
            FileKind::Requirements,
            FileKind::Inventory,
            FileKind::RoleArgSpec,
            FileKind::Molecule,
            FileKind::ExecutionEnvironment,
            FileKind::Changelog,
            FileKind::Yaml,
            FileKind::Unknown,
        ] {
            assert_eq!(FileKind::from_name(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn recognises_task_lists() {
        assert!(FileKind::Tasks.is_task_list());
        assert!(FileKind::Handlers.is_task_list());
        assert!(!FileKind::Playbook.is_task_list());
    }
}
