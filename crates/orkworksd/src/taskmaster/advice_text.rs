//! Producer-selected text validation; inert advice never grants action authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextPolicy {
    Legacy,
    InertAdvice,
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
            || super::assessment_model::has_completed_verification_claim(&value.to_ascii_lowercase(), prospective)
        {
            return Err("assessment contains unsafe or out-of-bounds text".into());
        }
        Ok(())
    }
}
