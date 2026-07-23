use crate::harness::integration::{
    IntegrationActivation, IntegrationContext, IntegrationCoverage, IntegrationDiagnostic,
    IntegrationError, IntegrationHandler, IntegrationOwnership, IntegrationRegistration,
    IntegrationStatus,
};

pub(crate) struct CodexHandler;
pub(crate) static HANDLER: CodexHandler = CodexHandler;

impl IntegrationHandler for CodexHandler {
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
        harness_id: "codex".into(), enabled: ctx.enabled, tool_detected: ctx.detected_tool.is_some(),
        registration: IntegrationRegistration::Unsupported, ownership: IntegrationOwnership::None,
        activation: IntegrationActivation::NotApplicable, coverage: IntegrationCoverage::None,
        diagnostics: vec![IntegrationDiagnostic { code: "installation_unsupported".into(), message: "Codex has no verified local-only hook configuration contract; OrkWorks will not edit .codex.".into(), action: None }], confirmation: None,
    }
}
