//! Rule profiles, ordered from least to most strict.

use std::fmt;

/// A cumulative rule profile, mirroring upstream `ansible-lint`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Profile {
    /// The minimum profile.
    #[default]
    Min,
    /// Basic.
    Basic,
    /// Moderate.
    Moderate,
    /// Safety.
    Safety,
    /// Shared.
    Shared,
    /// Production.
    Production,
}

impl Profile {
    /// Every profile in cumulative order.
    pub const ALL: [Self; 6] = [
        Self::Min,
        Self::Basic,
        Self::Moderate,
        Self::Safety,
        Self::Shared,
        Self::Production,
    ];

    /// Returns the upstream spelling of the profile.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Min => "min",
            Self::Basic => "basic",
            Self::Moderate => "moderate",
            Self::Safety => "safety",
            Self::Shared => "shared",
            Self::Production => "production",
        }
    }

    /// Returns `true` when self is at least as strict as `other`.
    #[must_use]
    pub const fn includes(self, other: Self) -> bool {
        self as u8 >= other as u8
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for Profile {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "min" => Ok(Self::Min),
            "basic" => Ok(Self::Basic),
            "moderate" => Ok(Self::Moderate),
            "safety" => Ok(Self::Safety),
            "shared" => Ok(Self::Shared),
            "production" => Ok(Self::Production),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Profile;

    #[test]
    fn parses_every_upstream_name() {
        for profile in Profile::ALL {
            assert_eq!(profile.as_str().parse::<Profile>(), Ok(profile));
        }
        assert!("none".parse::<Profile>().is_err());
    }

    #[test]
    fn profiles_are_cumulative() {
        assert!(Profile::Production.includes(Profile::Min));
        assert!(Profile::Basic.includes(Profile::Min));
        assert!(!Profile::Min.includes(Profile::Basic));
    }
}
