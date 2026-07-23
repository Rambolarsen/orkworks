use crate::harness::integration::{
    IntegrationActivation, IntegrationContext, IntegrationCoverage, IntegrationDiagnostic,
    IntegrationError, IntegrationHandler, IntegrationOwnership, IntegrationRegistration,
    IntegrationStatus,
};

pub(crate) struct OpenCodeHandler;
pub(crate) static HANDLER: OpenCodeHandler = OpenCodeHandler;

impl IntegrationHandler for OpenCodeHandler {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        Ok(unsupported(ctx))
    }
    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        Ok(unsupported(ctx))
    }
    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        Ok(unsupported(ctx))
    }
}

fn unsupported(ctx: &IntegrationContext<'_>) -> IntegrationStatus {
    IntegrationStatus {
        harness_id: "opencode".into(),
        enabled: ctx.enabled,
        tool_detected: ctx.detected_tool.is_some(),
        registration: IntegrationRegistration::Unsupported,
        ownership: IntegrationOwnership::None,
        activation: IntegrationActivation::Unknown,
        coverage: IntegrationCoverage::Limited,
        diagnostics: vec![IntegrationDiagnostic {
            code: "installation_unsupported".into(),
            message: "OpenCode plugin activation and an eligible local target are unverified; no plugin file is written.".into(),
            action: None,
        }],
        confirmation: None,
    }
}
