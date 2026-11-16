//! Producer-selected text validation; inert advice never grants action authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextPolicy {
    Legacy,
    InertAdvice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prospective_transit_is_not_completed_verification_only_for_inert_advice() {
        for text in [
            "An incorrect addition result would be detectable by the behavioral test rather than passing through an assertion unrelated to the output.",
            "Incorrect output would stop passing through the test undetected.",
            "An invalid result could be rejected instead of passing through the check.",
        ] {
            assert!(TextPolicy::InertAdvice.validate(text, 1000, true, true).is_ok(), "rejected {text:?}");
            assert!(TextPolicy::InertAdvice.validate(text, 1000, true, false).is_err(), "descriptive field accepted {text:?}");
            assert!(TextPolicy::Legacy.validate(text, 1000, true, true).is_err(), "legacy field accepted {text:?}");
        }
    }
}
impl TextPolicy {
    pub(crate) fn for_improvement_purpose(purpose: &str) -> Result<Self, String> {
        match purpose {
            "brain-improvement-v1" => Ok(Self::Legacy),
            "brain-improvement-v2" => Ok(Self::InertAdvice),
            _ => Err("Unsupported improvement text identity".into()),
        }
    }

    pub(crate) fn validate(
        self,
        value: &str,
        limit: usize,
        required: bool,
        prospective: bool,
    ) -> Result<(), String> {
        if self == Self::Legacy {
            return if prospective {
                super::assessment_model::validate_prospective_prose(value, limit, required)
            } else {
                super::assessment_model::validate_prose(value, limit, required)
            };
        }
        if value.chars().count() > limit
            || (required && value.trim().is_empty())
            || value.chars().any(|ch| ch.is_control() || matches!(ch, '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
            || super::assessment_model::has_completed_verification_claim(&value.to_ascii_lowercase(), prospective, self)
        {
            return Err("assessment contains unsafe or out-of-bounds text".into());
        }
        Ok(())
    }
}
