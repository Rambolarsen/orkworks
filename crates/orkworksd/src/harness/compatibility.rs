use serde::{Deserialize, Serialize};

use super::definition::{IntegrationBinding, SessionSignalBinding};

/// A closed, code-owned compatibility contract that a custom harness may use.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CompatibilityProfile {
    Copilot,
}

/// Read-only bindings derived from a compatibility profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompatibilityMetadata {
    pub profile: Option<CompatibilityProfile>,
    pub session_signals: Option<SessionSignalBinding>,
    pub integration: Option<IntegrationBinding>,
}

/// Derives compiled bindings from the closed compatibility-profile allowlist.
pub(crate) fn derive_compatibility_metadata(
    profile: Option<CompatibilityProfile>,
) -> CompatibilityMetadata {
    match profile {
        Some(CompatibilityProfile::Copilot) => CompatibilityMetadata {
            profile,
            session_signals: Some(SessionSignalBinding::Copilot),
            integration: Some(IntegrationBinding::Copilot),
        },
        None => CompatibilityMetadata {
            profile: None,
            session_signals: None,
            integration: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{IntegrationBinding, SessionSignalBinding};

    #[test]
    fn copilot_profile_derives_the_closed_copilot_bindings() {
        let metadata = derive_compatibility_metadata(Some(CompatibilityProfile::Copilot));

        assert_eq!(metadata.profile, Some(CompatibilityProfile::Copilot));
        assert_eq!(metadata.integration, Some(IntegrationBinding::Copilot));
        assert_eq!(
            metadata.session_signals,
            Some(SessionSignalBinding::Copilot)
        );
    }

    #[test]
    fn no_profile_derives_no_bindings() {
        let metadata = derive_compatibility_metadata(None);

        assert_eq!(metadata.profile, None);
        assert_eq!(metadata.integration, None);
        assert_eq!(metadata.session_signals, None);
    }
}
