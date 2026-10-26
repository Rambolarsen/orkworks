//! Fixed diagnostic categories. Never persist provider messages or model text.
use crate::providers::{ProviderOperationError, ProviderOperationErrorCode};

pub(super) fn provider_code(error: &ProviderOperationError) -> &'static str {
    use ProviderOperationErrorCode::*;
    match error.code {
        Timeout => "provider_timeout",
        Unauthorized => "provider_authentication",
        ModelFailure => "provider_model",
        UnsupportedCapability => "provider_unsupported",
        Malformed => "provider_request",
        StaleGeneration | VerificationRequired => "inputs_changed",
        ProviderFailure
            if error
                .message
                .starts_with("inference-only CLI invocation failed (request validation error") =>
        {
            "provider_request"
        }
        UnknownProvider | ProviderFailure => "provider_unavailable",
    }
}

pub(super) fn response_code(error: &str) -> &'static str {
    match error {
        "assessment contains unsafe or out-of-bounds text" => "invalid_prose",
        "Invalid readiness citations or concept"
        | "assessment cites an unknown repository fact"
        | "assessment cites an unknown knowledge page" => "invalid_citations",
        "Invalid readiness rating or unassessed reason"
        | "Invalid readiness version or coverage" => "invalid_rating",
        _ => "invalid_response",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_diagnostics_use_typed_codes_without_copying_private_messages() {
        for (code, expected) in [
            (ProviderOperationErrorCode::Timeout, "provider_timeout"),
            (
                ProviderOperationErrorCode::Unauthorized,
                "provider_authentication",
            ),
            (ProviderOperationErrorCode::ModelFailure, "provider_model"),
            (
                ProviderOperationErrorCode::StaleGeneration,
                "inputs_changed",
            ),
            (
                ProviderOperationErrorCode::ProviderFailure,
                "provider_unavailable",
            ),
        ] {
            assert_eq!(
                provider_code(&ProviderOperationError {
                    code,
                    message: "/secret/account provider response".into()
                }),
                expected
            );
        }
        assert_eq!(
            response_code("/secret/account model response"),
            "invalid_response"
        );
    }
}
